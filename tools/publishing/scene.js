// Quiet page atmosphere and a shared, persistent motion preference.
// The landing's hand/orb/halo/tide is rendered separately by hero-scene.js.
(() => {
  const canvas=document.querySelector('#page-particles'),toggle=document.querySelector('#motion-toggle');
  if(!canvas||!toggle)return;
  const context=canvas.getContext('2d');if(!context)return;
  const mode=document.body.dataset.atmosphere||'browse';
  const settings={hero:{count:110,alpha:.36,drift:.3},browse:{count:260,alpha:.66,drift:.45},reading:{count:150,alpha:.42,drift:.12}}[mode];
  const reduced=matchMedia('(prefers-reduced-motion:reduce)'),fine=matchMedia('(hover:hover) and (pointer:fine)');
  let paused=reduced.matches,frame=0,last=0,time=0,width=0,height=0,light=false;
  const pointer={x:-1000,y:-1000,active:false},camera={x:0,y:0};
  let seed=12801;
  const random=()=>{seed=(seed*1664525+1013904223)>>>0;return seed/4294967296;};
  const points=Array.from({length:260},()=>({x:random(),y:random(),depth:.2+random()*.8,size:random(),phase:random()*6.28,dx:0,dy:0}));
  const wrap=(value,size)=>((value%size)+size)%size;
  function readPreference(){try{const value=localStorage.getItem('nc-motion');return value==='paused'||(value!=='playing'&&reduced.matches);}catch{return reduced.matches;}}
  function paint(step=0){
    context.clearRect(0,0,width,height);
    if(step){camera.x+=(pointer.active?(pointer.x/width-.5)*2-camera.x:-camera.x)*.045;camera.y+=(pointer.active?(pointer.y/height-.5)*2-camera.y:-camera.y)*.045;}
    const stride=fine.matches?1:2;
    for(let i=0;i<settings.count;i+=stride){
      const p=points[i],x=wrap(p.x*(width+60)+Math.sin(time*.07+p.phase)*7+camera.x*p.depth*8,width+60)-30;
      const y=wrap(p.y*(height+60)+time*p.depth*settings.drift,height+60)-30;
      const distance=Math.hypot(x-pointer.x,y-pointer.y);
      const force=pointer.active&&fine.matches&&mode!=='reading'&&!paused?Math.max(0,1-distance/110):0;
      if(step){p.dx+=((x-pointer.x)*force*.11-p.dx)*.035;p.dy+=((y-pointer.y)*force*.11-p.dy)*.035;}
      const alpha=(.09+p.depth*.17+p.size*.06)*settings.alpha*(light?.72:1);
      context.fillStyle=`rgba(${light?(i%6===0?'130,91,60':'61,77,65'):(i%6===0?'212,170,129':'222,227,217')},${alpha})`;
      context.beginPath();context.arc(x+p.dx,y+p.dy,.25+p.size*(.6+p.depth*.7),0,Math.PI*2);context.fill();
    }
  }
  function tick(now){frame=0;if(paused||document.hidden)return;if(last&&now-last<32){frame=requestAnimationFrame(tick);return;}const delta=last?Math.min((now-last)/1000,.07):0;last=now;time+=delta;paint(delta);frame=requestAnimationFrame(tick);}
  function reconcile(){cancelAnimationFrame(frame);frame=0;last=0;if(!document.hidden){paint();if(!paused)frame=requestAnimationFrame(tick);}}
  function applyPause(value){
    paused=value;document.documentElement.dataset.motion=paused?'paused':'playing';
    if(paused)document.querySelector('.cinematic-hero')?.getAnimations({subtree:true}).forEach(animation=>{try{animation.finish();}catch{animation.cancel();}});
    toggle.setAttribute('aria-pressed',String(paused));toggle.querySelector('.motion-label').textContent=paused?'Play motion':'Pause motion';toggle.querySelector('.motion-icon').textContent=paused?'▷':'Ⅱ';
    dispatchEvent(new CustomEvent('nc:motion',{detail:{paused}}));reconcile();
  }
  function resize(){width=innerWidth;height=innerHeight;const ratio=Math.min(devicePixelRatio||1,1.5);canvas.width=Math.round(width*ratio);canvas.height=Math.round(height*ratio);context.setTransform(ratio,0,0,ratio,0,0);paint();}
  function palette(){light=document.documentElement.dataset.theme==='light';if(paused)paint();}
  toggle.addEventListener('click',()=>{const next=!paused;try{localStorage.setItem('nc-motion',next?'paused':'playing');}catch{}applyPause(next);});
  addEventListener('storage',event=>{if(event.key==='nc-motion')applyPause(readPreference());});
  addEventListener('pageshow',()=>applyPause(readPreference()));
  reduced.addEventListener('change',()=>applyPause(readPreference()));
  addEventListener('pointermove',event=>{pointer.x=event.clientX;pointer.y=event.clientY;pointer.active=true;},{passive:true});
  document.addEventListener('pointerleave',()=>{pointer.active=false;});
  document.addEventListener('visibilitychange',reconcile);
  addEventListener('resize',resize,{passive:true});
  addEventListener('pagehide',()=>cancelAnimationFrame(frame));
  new MutationObserver(palette).observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});
  document.body.classList.add('has-atmosphere');toggle.closest('.motion-controls').hidden=false;
  resize();palette();applyPause(readPreference());
})();
