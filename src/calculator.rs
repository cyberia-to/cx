//! the ground-lease pricer, ported from the graph.
//!
//! the model is unchanged from `cyber-valley/strategy/lease-pricer.md`
//! (commit 5387526): the premium takes p of the leasehold's economics
//! upfront, the remainder returns as indexed rent discounted at
//! `r = r_base + spread·(1−p)` — a smaller premium shifts value into the rent
//! stream and raises the rate, pricing the tenant's default risk. after year
//! one the rent moves with the indexation alone: the protocol page carries no
//! market review, so the pricer carries none either.
//!
//! the model is reparameterised on the leasehold's own value rather than the
//! freehold behind it: the product is a right of use, never a title. labels are
//! english and the palette is the site's.

/// the whole block. `indexation_default` seeds the indexation slider with what
/// a lease of the base date actually paid per year, clamped into the collar
/// band the slider spans; `base_date` names that date in the hint.
pub fn html(indexation_default: &str, base_date: &str) -> String {
    TEMPLATE
        .replace("__INDEXATION__", indexation_default)
        .replace("__BASE__", base_date)
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
const P={L:100000,T:25,p:20,rBase:9,spread:5,cpi:__INDEXATION__};
const fmt=v=>v>=1000?'$'+(v/1000).toFixed(v>=100000?0:1)+'k':'$'+Math.round(v);
const full=v=>'$'+Math.round(v).toLocaleString('en-US');
const pct=(v,d=1)=>v.toFixed(d)+'%';
// the client's side is theirs to move, and the indexation is the scenario to
// stress-test. what the estate underwrites and what the world does stay put
const LIVE=['L','T','p','cpi'];
const DEFS=[
 ['deal','Your lease'],
 ['L','Leasehold value today',20000,2000000,10000,'$','What the right to use the parcel for the term is worth — the product itself, priced directly'],
 ['T','Term',5,99,1,' yr','How many years the lease runs'],
 ['p','Premium (share of economics)',0,100,5,'%','Yours to choose: how much of the lease is paid at signing rather than carried as rent. 100% is the upfront instrument, 0% is pure rent'],
 ['world','What the world does'],
 ['cpi','Indexation',-15,35,0.5,'%','The band is the collar of the protocol: rent may rise at most 35% and fall at most 15% in a year. The default is what a lease signed on __BASE__ actually paid per year, collar and floor applied'],
 ['estate','How the estate prices it'],
 ['rBase','Base rate (at 100% premium)',5,18,0.5,'%','The estate discount rate. Not a tenant dial — shown so the price can be checked rather than trusted'],
 ['spread','Risk spread (at 0% premium)',0,10,0.5,'%','What the estate charges for carrying default risk as the premium shrinks']];
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
+'<p class=note>The rent is the index and nothing else: fixed quantities, priced each year within the collar. No market review sits on top.</p></div></div></div>';

function calc(){
 const pf=P.p/100, rAdj=(P.rBase+P.spread*(1-pf))/100, cpi=P.cpi/100;
 const totalPV=P.L, premium=pf*totalPV, remainder=totalPV-premium;
 let A=0; for(let t=1;t<=P.T;t++) A+=Math.pow(1+cpi,t-1)/Math.pow(1+rAdj,t);
 const rent1=remainder/A, yld=rent1/P.L*100;
 const rows=[]; let bi=rent1;
 for(let t=1;t<=P.T;t++){
  if(t>1) bi*=1+cpi;
  rows.push([t,bi]);
 }
 return{rAdj:rAdj*100,premium,totalPV,rent1,yld,rows};
}
function stat(l,v,s,c){return '<div class=stat><div class=l>'+l+'</div><div class=v'+(c?' style=color:'+c:'')+'>'+v+'</div>'+(s?'<div class=s>'+s+'</div>':'')+'</div>'}
function chart(rows){
 const W=640,H=290,L=54,R=10,Tp=10,B=24,pw=W-L-R,ph=H-Tp-B;
 const mx=Math.max(...rows.map(r=>r[1]))*1.06;
 const x=t=>L+pw*(t-1)/Math.max(rows.length-1,1), y=v=>Tp+ph*(1-v/mx);
 const line=(i,col)=>'<polyline fill=none stroke='+col+' stroke-width=2.2'
  +' points="'+rows.map(r=>x(r[0]).toFixed(1)+','+y(r[i]).toFixed(1)).join(' ')+'"/>';
 let ticks='';
 for(let i=0;i<=4;i++){const v=mx*i/4,yy=y(v);
  ticks+='<line x1='+L+' y1='+yy+' x2='+(W-R)+' y2='+yy+' stroke=#1f1f1f stroke-width=0.5></line>'
  +'<text x='+(L-6)+' y='+(yy+3)+' text-anchor=end>'+fmt(v)+'</text>';}
 const step=Math.ceil(rows.length/8);
 for(let t=1;t<=rows.length;t+=step) ticks+='<text x='+x(t)+' y='+(H-6)+' text-anchor=middle>'+t+'</text>';
 return '<svg viewBox="0 0 '+W+' '+H+'" style="width:100%;height:auto">'+ticks
  +line(1,'#79ff4f')+'</svg>';
}
function render(){
 for(const d of DEFS){if(d.length===2)continue;const id=d[0];
  document.getElementById('v_'+id).textContent=(d[5]==='$'?full(P[id]):P[id]+d[5]);}
 const c=calc();
 document.getElementById('st1').innerHTML=
  stat('Premium (first payment)',fmt(c.premium),P.p+'% of '+fmt(c.totalPV)+' of leasehold value','var(--jade)')
  +stat('Rent, year 1',fmt(c.rent1)+'/yr','yield '+pct(c.yld,2));
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
        let h = html("4", "2022-01-07");
        // every input of the original model survives the port
        for id in ["L", "T", "p", "rBase", "spread", "cpi"] {
            assert!(h.contains(&format!("['{id}'")), "missing control {id}");
        }
        // the pricing identities, unchanged
        assert!(h.contains("rAdj=(P.rBase+P.spread*(1-pf))/100"));
        assert!(h.contains("A+=Math.pow(1+cpi,t-1)/Math.pow(1+rAdj,t)"));
        // the protocol has no market review, so neither does the pricer
        assert!(!h.contains("target=") && !h.contains("Review every"));
    }

    #[test]
    fn is_published_in_english() {
        // the graph copy was russian; a published page is not
        assert!(!html("4", "2022-01-07").chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)));
    }

    #[test]
    fn scopes_its_own_ids() {
        // the page already owns #chart; the pricer must not collide with it
        let h = html("4", "2022-01-07");
        assert!(h.contains("lchart"));
        assert!(!h.contains("getElementById('chart')"));
    }
}

#[cfg(test)]
mod default_tests {
    use super::*;

    #[test]
    fn the_indexation_default_is_injected_and_the_band_is_the_collar() {
        let h = html("35", "2022-01-07");
        assert!(h.contains("cpi:35"), "default not seeded");
        assert!(!h.contains("__INDEXATION__"), "placeholder survived");
        assert!(!h.contains("__BASE__"), "base placeholder survived");
        assert!(h.contains("signed on 2022-01-07"), "base date not in the hint");
        assert!(h.contains("'Indexation',-15,35,0.5"), "slider does not span the collar");
    }
}

#[cfg(test)]
mod vocabulary_tests {
    use super::*;

    #[test]
    fn the_pricer_never_speaks_of_freehold() {
        let h = html("35", "2022-01-07");
        // a right of use is the product; quoting a freehold here would price a
        // thing that is not for sale
        assert!(h.contains("'Leasehold value today'"));
        assert!(!h.to_lowercase().contains("freehold"));
        assert!(!h.contains("Relativity"));
    }
}

#[cfg(test)]
mod audience_tests {
    use super::*;

    #[test]
    fn the_term_opens_at_the_standard_lease() {
        assert!(html("35", "2022-01-07").contains("T:25"), "term should default to 25 years");
    }

    #[test]
    fn the_client_moves_their_own_terms_and_the_scenario() {
        let h = html("35", "2022-01-07");
        // price, term and premium are the client's to set; the indexation is
        // the scenario. the estate's underwriting is not negotiable here and
        // must not look like a control.
        assert!(h.contains("LIVE=['L','T','p','cpi']"), "the client lost a control");
        assert!(h.contains("for(const id of LIVE)"), "listeners are not bound to LIVE");
        assert!(h.contains("live?'<input type=range"), "the control is not gated on LIVE");
        assert!(h.contains("row.fixed"), "fixed rows are not styled as stated figures");
    }

    #[test]
    fn the_dials_are_grouped_by_who_holds_them() {
        let h = html("35", "2022-01-07");
        // a reader must be able to tell the deal from the estate's underwriting
        let choose = h.find("Your lease").expect("client group");
        let world = h.find("What the world does").expect("world group");
        let estate = h.find("How the estate prices it").expect("estate group");
        assert!(choose < world && world < estate, "groups out of order");
        // the underwriting dials say plainly that they are not the tenant's
        assert!(h.contains("Not a tenant dial"));
    }
}

#[cfg(test)]
mod scope_tests {
    use super::*;

    #[test]
    fn the_estate_underwriting_stays_off_the_client_page() {
        let h = html("35", "2022-01-07");
        // what the estate earns from the review is its own business; on a
        // client page it reads as a bill, and the nominal sums over a quarter
        // century read as a price they are not
        for gone in [
            "What the review buys",
            "PV gained from review",
            "Nominal rent over the term",
            "Lease at the end",
            "PV (present value) of the deal",
        ] {
            assert!(!h.contains(gone), "{gone} is still on the page");
        }
        // premium, rent and the path remain
        assert!(h.contains("Premium (first payment)"));
        assert!(h.contains("Rent, year 1"));
        assert!(h.contains("Rent path, $/yr"));
        // one line, and a sentence saying what it is
        assert!(h.contains("The rent is the index and nothing else"));
        assert!(!h.contains("Amber adds the review clause"));
        // an unescaped apostrophe here once tore the script in half
        assert!(!h.contains("\\'"), "double-escaped quote in the emitted script");
    }
}
