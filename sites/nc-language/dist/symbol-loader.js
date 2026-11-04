// The project symbol is secondary: load its GPU engine only near the viewport.
const stage=document.querySelector('.cinematic-hero');
const status=document.querySelector('#scene-status');
if(stage&&status){
 let near=false,loaded=false;
 const reduced=matchMedia('(prefers-reduced-motion:reduce)');
 async function load(){
  if(loaded||!near)return;
  if(new URLSearchParams(location.search).get('visual')==='fallback'){status.dataset.renderer='fallback';return;}
  if(document.documentElement.dataset.motion==='paused'||(reduced.matches&&document.documentElement.dataset.motion!=='playing')){status.dataset.renderer='still';return;}
  loaded=true;
  try{await import('./hero-scene.js?v=d7647e5ae1f5');}catch(error){status.dataset.renderer='fallback';console.warn('nC symbol unavailable; the source and documentation remain available.',error);}
 }
 status.dataset.renderer='waiting';
 const observer=new IntersectionObserver(entries=>{near=entries[0].isIntersecting;load();},{rootMargin:'240px'});observer.observe(stage);
 addEventListener('nc:motion',load);
 addEventListener('pagehide',()=>observer.disconnect());
 addEventListener('pageshow',event=>{if(event.persisted&&!loaded)observer.observe(stage);});
}
