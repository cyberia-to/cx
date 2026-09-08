//! the ground-lease pricer, ported from the graph.
//!
//! the model is unchanged from `cyber-valley/strategy/lease-pricer.md`
//! (commit 5387526): the premium takes p of the leasehold's economics
//! upfront, the remainder returns as indexed rent discounted at
//! `r = r_base + spread·(1−p)` — a smaller premium shifts value into the rent
//! stream and raises the rate, pricing the tenant's default risk. review every
//! N years resets rent to `max(indexed path, X% × market freehold)`, where X is
//! the initial yield, so the lessor keeps the same position in the land forever.
//!
//! the model is reparameterised on the leasehold's own value rather than the
//! freehold behind it: the product is a right of use, never a title, and the
//! freehold cancels out of the review target anyway — `X% × land` with
//! `X = rent₁/freehold` is just the first year's rent carried forward at g.
//! labels are english and the palette is the site's; the rent path is
//! arithmetically what it was.

/// the whole block. `indexation_default` seeds the indexation slider with what
/// the index actually delivered, clamped into the collar band it now spans.
pub fn html(indexation_default: &str) -> String {
    TEMPLATE.replace("__INDEXATION__", indexation_default)
}

const TEMPLATE: &str = r##"<div id="lcalc"></div>
<style>
#lcalc{--s1:#0a0a0a;--s2:#111;--ln:#1f1f1f;--tx:#f2f2f2;--mut:#8a8a8a;--jade:#79ff4f;--amb:#c98500;--red:#e06060;
 background:var(--s1);color:var(--tx);font-family:'Play',sans-serif;border:1px solid var(--ln);border-radius:12px;padding:16px}
#lcalc .cols{display:flex;gap:16px;flex-wrap:wrap}
#lcalc .panel{background:var(--s2);border:1px solid var(--ln);border-radius:10px;padding:14px}
#lcalc .ctrl{flex:1 1 240px;min-width:220px}
#lcalc .out{flex:1 1 260px;min-width:240px;display:flex;flex-direction:column;gap:12px}
#lcalc h4{font-size:13px;color:var(--tx);letter-spacing:1.2px;text-transform:uppercase;margin:14px 0 10px;font-weight:400}
#lcalc h4:first-child{margin-top:0}
#lcalc .row{margin-bottom:12px}
#lcalc .row.fixed{padding-bottom:10px;border-bottom:1px solid var(--ln)}
#lcalc .row.fixed .val{color:var(--tx)}
#lcalc .row.live .val{color:var(--jade)}
#lcalc .row .top{display:flex;justify-content:space-between;align-items:baseline;margin-bottom:3px;gap:8px}
#lcalc label{font-size:12px;color:var(--mut)}
#lcalc .val{font-size:13px;font-variant-numeric:tabular-nums;white-space:nowrap}
#lcalc input[type=range]{width:100%;accent-color:var(--jade);height:3px;cursor:pointer;margin:0}
#lcalc .hint{font-size:11px;color:var(--mut);margin-top:3px;line-height:1.4}
#lcalc .stats{display:flex;gap:10px;flex-wrap:wrap}
#lcalc .stat{background:var(--s2);border:1px solid var(--ln);border-radius:8px;padding:10px 12px;flex:1;min-width:118px}
#lcalc .stat .l{font-size:11px;color:var(--mut);margin-bottom:3px}
#lcalc .stat .v{font-size:18px;font-weight:700;font-variant-numeric:tabular-nums}
#lcalc .stat .s{font-size:11px;color:var(--mut);margin-top:2px}
#lcalc .note{font-size:12px;color:var(--mut);line-height:1.55;margin:10px 0 0}
#lcalc svg text{font-size:10px;fill:var(--mut);font-variant-numeric:tabular-nums}
#lcalc .legend{font-size:11px;color:var(--mut);display:flex;gap:14px;flex-wrap:wrap;margin:6px 0 0 4px}
#lcalc .legend i{display:inline-block;width:14px;height:2.5px;vertical-align:middle;margin-right:5px}
</style>
<script>
(function(){
const root=document.getElementById('lcalc');
const P={L:100000,T:25,p:20,rBase:9,spread:5,cpi:__INDEXATION__,g:10,N:5};
const fmt=v=>v>=1000?'$'+(v/1000).toFixed(v>=100000?0:1)+'k':'$'+Math.round(v);
const full=v=>'$'+Math.round(v).toLocaleString('en-US');
const pct=(v,d=1)=>v.toFixed(d)+'%';
// only the indexation moves: everything else is the standard deal and the
// estate's underwriting, shown as the fixed figures they are
const LIVE=['cpi'];
const DEFS=[
 ['deal','Your lease'],
 ['L','Leasehold value today',20000,2000000,10000,'$','What the right to use the parcel for the term is worth — the product itself, priced directly'],
 ['T','Term',5,99,1,' yr','The standard term'],
 ['p','Premium (share of economics)',0,100,5,'%','How much of the lease is paid at signing rather than carried as rent. 100% is the upfront instrument, 0% is pure rent'],
 ['world','What the world does'],
 ['cpi','Indexation',-15,35,0.5,'%','The band is the collar of the protocol: rent may rise at most 35% and fall at most 15% in a year. The default is what the index delivered over the published decade, which ran hotter than the collar allows'],
 ['g','Land growth g',0,20,0.5,'%','What the parcel appreciates at. Nobody sets this; Bali has historically outrun CPI'],
 ['estate','How the estate prices it'],
 ['rBase','Base rate (at 100% premium)',5,18,0.5,'%','The estate discount rate. Not a tenant dial — shown so the price can be checked rather than trusted'],
 ['spread','Risk spread (at 0% premium)',0,10,0.5,'%','What the estate charges for carrying default risk as the premium shrinks'],
 ['N','Review every',1,15,1,' yr','Rent = max(indexed path, the first year carried forward at g); the lessor keeps the same position in the parcel however far the index falls behind']];
let html='';
for(const d of DEFS){
 if(d.length===2){html+='<h4>'+d[1]+'</h4>';continue}
 const[id,lab,mn,mx,st,suf,hint]=d;
 const live=LIVE.includes(id);
 html+='<div class="row'+(live?' live':' fixed')+'"><div class=top><label>'+lab+'</label><span class=val id=v_'+id+'></span></div>'
 +(live?'<input type=range id=in_'+id+' min='+mn+' max='+mx+' step='+st+' value='+P[id]+'>':'')
 +(hint?'<div class=hint>'+hint+'</div>':'')+'</div>';
}
root.innerHTML='<div class=cols><div class="panel ctrl">'+html+'</div>'
+'<div class=out><div class=stats id=st1></div>'
+'<div class=panel><h4>Rent path, $/yr</h4><div id=lchart></div>'
+'<div class=legend><span><i style="background:var(--mut)"></i>rent tracking the parcel (target)</span>'
+'<span><i style="background:var(--red)"></i>index only</span>'
+'<span><i style="background:var(--amb)"></i>index + review</span></div></div>'
+'<div class=panel><h4>What the review buys</h4><div class=stats id=st2></div>'
+'<p class=note>The red line is rent that is indexed but whose share of the asset melts away when g &gt; the index. '
+'The amber line is pulled back at each review to X% of current land value and indexed from the new base. '
+'When g equals the index the two coincide: the review costs the tenant nothing and is pure insurance.</p></div></div></div>';

function calc(){
 const pf=P.p/100, rAdj=(P.rBase+P.spread*(1-pf))/100, cpi=P.cpi/100, g=P.g/100;
 const totalPV=P.L, premium=pf*totalPV, remainder=totalPV-premium;
 let A=0; for(let t=1;t<=P.T;t++) A+=Math.pow(1+cpi,t-1)/Math.pow(1+rAdj,t);
 const rent1=remainder/A, yld=rent1/P.L*100;
 const rows=[]; let bi=rent1,br=rent1,pvI=0,pvR=0,nomI=0,nomR=0;
 for(let t=1;t<=P.T;t++){
  if(t>1){bi*=1+cpi;br*=1+cpi}
  // the review target keeps the lessor's share of the parcel: the first
  // year's rent carried forward at the growth rate
  const target=rent1*Math.pow(1+g,t-1);
  if(t>1&&(t-1)%P.N===0) br=Math.max(br,target);
  rows.push([t,bi,br,target]);
  pvI+=bi/Math.pow(1+rAdj,t); pvR+=br/Math.pow(1+rAdj,t); nomI+=bi; nomR+=br;
 }
 return{rAdj:rAdj*100,premium,totalPV,rent1,yld,rows,pvI,pvR,nomI,nomR,
  leaseEnd:P.L*Math.pow(1+g,P.T-1),takeI:premium+pvI,takeR:premium+pvR};
}
function stat(l,v,s,c){return '<div class=stat><div class=l>'+l+'</div><div class=v'+(c?' style=color:'+c:'')+'>'+v+'</div>'+(s?'<div class=s>'+s+'</div>':'')+'</div>'}
function chart(rows){
 const W=640,H=290,L=54,R=10,Tp=10,B=24,pw=W-L-R,ph=H-Tp-B;
 const mx=Math.max(...rows.map(r=>Math.max(r[2],r[3])))*1.06;
 const x=t=>L+pw*(t-1)/Math.max(rows.length-1,1), y=v=>Tp+ph*(1-v/mx);
 const line=(i,col,dash)=>'<polyline fill=none stroke='+col+' stroke-width='+(i===2?2.2:1.5)+(dash?' stroke-dasharray="3 4"':'')
  +' points="'+rows.map(r=>x(r[0]).toFixed(1)+','+y(r[i]).toFixed(1)).join(' ')+'"/>';
 let ticks='';
 for(let i=0;i<=4;i++){const v=mx*i/4,yy=y(v);
  ticks+='<line x1='+L+' y1='+yy+' x2='+(W-R)+' y2='+yy+' stroke=#1f1f1f stroke-width=0.5></line>'
  +'<text x='+(L-6)+' y='+(yy+3)+' text-anchor=end>'+fmt(v)+'</text>';}
 const step=Math.ceil(rows.length/8);
 for(let t=1;t<=rows.length;t+=step) ticks+='<text x='+x(t)+' y='+(H-6)+' text-anchor=middle>'+t+'</text>';
 return '<svg viewBox="0 0 '+W+' '+H+'" style="width:100%;height:auto">'+ticks
  +line(3,'#8a8a8a',1)+line(1,'#e06060')+line(2,'#c98500')+'</svg>';
}
function render(){
 for(const d of DEFS){if(d.length===2)continue;const id=d[0];
  document.getElementById('v_'+id).textContent=(d[5]==='$'?full(P[id]):P[id]+d[5]);}
 const c=calc(), gain=c.takeR-c.takeI;
 document.getElementById('st1').innerHTML=
  stat('Premium (first payment)',fmt(c.premium),P.p+'% of '+fmt(c.totalPV)+' of leasehold value','var(--jade)')
  +stat('Rent, year 1',fmt(c.rent1)+'/yr','yield '+pct(c.yld,2)+' · rate '+pct(c.rAdj))
  +stat('PV of the deal',fmt(c.takeR),'vs '+fmt(c.takeI)+' without review');
 document.getElementById('st2').innerHTML=
  stat('PV gained from review','+'+fmt(gain),'+'+pct(gain/c.takeI*100)+' on the deal','var(--amb)')
  +stat('Nominal rent over the term',fmt(c.nomR),'vs '+fmt(c.nomI)+' without review')
  +stat('Lease at the end',fmt(c.leaseEnd),'growth '+P.g+'%/yr over '+P.T+' years');
 document.getElementById('lchart').innerHTML=chart(c.rows);
}
for(const id of LIVE){
 document.getElementById('in_'+id).addEventListener('input',e=>{P[id]=parseFloat(e.target.value);render()});}
render();
})();
</script>"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_the_model_and_its_controls() {
        let h = html("4");
        // every input of the original model survives the port
        for id in ["L", "T", "p", "rBase", "spread", "cpi", "g", "N"] {
            assert!(h.contains(&format!("['{id}'")), "missing control {id}");
        }
        // the pricing identities, unchanged
        assert!(h.contains("rAdj=(P.rBase+P.spread*(1-pf))/100"));
        assert!(h.contains("A+=Math.pow(1+cpi,t-1)/Math.pow(1+rAdj,t)"));
        assert!(h.contains("br=Math.max(br,target)"));
    }

    #[test]
    fn is_published_in_english() {
        // the graph copy was russian; a published page is not
        assert!(!html("4").chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)));
    }

    #[test]
    fn scopes_its_own_ids() {
        // the page already owns #chart; the pricer must not collide with it
        let h = html("4");
        assert!(h.contains("lchart"));
        assert!(!h.contains("getElementById('chart')"));
    }
}

#[cfg(test)]
mod default_tests {
    use super::*;

    #[test]
    fn the_indexation_default_is_injected_and_the_band_is_the_collar() {
        let h = html("35");
        assert!(h.contains("cpi:35"), "default not seeded");
        assert!(!h.contains("__INDEXATION__"), "placeholder survived");
        assert!(h.contains("'Indexation',-15,35,0.5"), "slider does not span the collar");
    }
}

#[cfg(test)]
mod vocabulary_tests {
    use super::*;

    #[test]
    fn the_pricer_never_speaks_of_freehold() {
        let h = html("35");
        // a right of use is the product; quoting a freehold here would price a
        // thing that is not for sale
        assert!(h.contains("'Leasehold value today'"));
        assert!(!h.to_lowercase().contains("freehold"));
        assert!(!h.contains("Relativity"));
        // the review target is the reparameterised identity
        assert!(h.contains("target=rent1*Math.pow(1+g,t-1)"));
    }
}

#[cfg(test)]
mod audience_tests {
    use super::*;

    #[test]
    fn the_term_opens_at_the_standard_lease() {
        assert!(html("35").contains("T:25"), "term should default to 25 years");
    }

    #[test]
    fn only_the_indexation_can_be_moved() {
        let h = html("35");
        // a slider invites negotiation; the deal and the underwriting are not
        // negotiable here, so they must not look like controls. ids are built
        // by concatenation at runtime, so assert on what the template emits.
        assert_eq!(h.matches("<input type=range").count(), 1, "more than one control");
        assert!(h.contains("LIVE=['cpi']"), "the live list is not just the index");
        assert!(h.contains("for(const id of LIVE)"), "listeners are not bound to LIVE");
        assert!(h.contains("live?'<input type=range"), "the control is not gated on LIVE");
        assert!(h.contains("row.fixed"), "fixed rows are not styled as stated figures");
    }

    #[test]
    fn the_dials_are_grouped_by_who_holds_them() {
        let h = html("35");
        // a reader must be able to tell the deal from the estate's underwriting
        let choose = h.find("Your lease").expect("client group");
        let world = h.find("What the world does").expect("world group");
        let estate = h.find("How the estate prices it").expect("estate group");
        assert!(choose < world && world < estate, "groups out of order");
        // the underwriting dials say plainly that they are not the tenant's
        assert!(h.contains("Not a tenant dial"));
    }
}
