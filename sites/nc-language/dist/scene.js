// A decorative, deterministic particle study. The language content is ordinary HTML.
(() => {
  const canvas=document.querySelector('#continuity-canvas');
  if(!canvas)return;
  const ctx=canvas.getContext('2d');
  if(!ctx)return;
  const figure=canvas.closest('figure'),toggle=document.querySelector('#motion-toggle');
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  let paused=reduced.matches,inView=true,frame=0,last=0,time=0,width=0,height=0;
  let seed=12801;
  const random=()=>{seed=(seed*1664525+1013904223)>>>0;return seed/4294967296;};
  const points=Array.from({length:2100},()=>({u:random()*Math.PI*2,v:random()*Math.PI*2,spread:random(),size:random(),phase:random()*Math.PI*2}));
  const field=Array.from({length:1000},()=>({x:random(),z:random(),size:random(),phase:random()*6.28}));
  const stars=Array.from({length:65},()=>({x:random(),y:random(),size:random()}));
  function paint(){
    if(!width||!height)return;
    ctx.clearRect(0,0,width,height);
    const scale=Math.min(width/620,height/610),cx=width*.51,cy=height*.43;
    ctx.save();ctx.translate(cx,cy);ctx.scale(scale,scale);
    const glow=ctx.createRadialGradient(0,15,20,0,15,280);
    glow.addColorStop(0,'rgba(219,219,212,0.014)');glow.addColorStop(.6,'rgba(229,235,224,0.025)');glow.addColorStop(1,'rgba(230,230,223,0)');
    ctx.fillStyle=glow;ctx.fillRect(-360,-310,720,700);
    for(const star of stars){ctx.fillStyle=`rgba(222,223,214,${.08+star.size*.2})`;ctx.fillRect((star.x-.5)*640,(star.y-.5)*520,star.size+0.35,star.size+0.35);}
    // A continuous torus, projected into a slightly tilted plane.
    const turn=time*.055,cos=Math.cos(turn),sin=Math.sin(turn);
    for(const p of points){
      const u=p.u+time*.028,rad=162+Math.cos(p.v)*22+(p.spread-.5)*18;
      const x=Math.cos(u)*rad,y=Math.sin(u)*rad,z=Math.sin(p.v)*27;
      const xx=x*cos+z*sin,zz=-x*sin+z*cos;
      const yy=y*.96+zz*.38,depth=(zz+180)/360;
      const perspective=700/(700+zz),px=xx*perspective,py=yy*perspective;
      const alpha=(.13+p.size*.52)*(0.6+depth*.4)*(0.91+Math.sin(time*.6+p.phase)*.09);
      ctx.fillStyle=`rgba(236,237,227,${alpha})`;
      ctx.beginPath();ctx.arc(px,py,.35+p.size*.72,0,Math.PI*2);ctx.fill();
    }
    // A soft light at the horizon; copper particles describe the material plane.
    const horizon=ctx.createRadialGradient(6,192,1,6,192,140);
    horizon.addColorStop(0,'rgba(255,231,191,.33)');horizon.addColorStop(.08,'rgba(241,186,137,.13)');horizon.addColorStop(.4,'rgba(189,131,92,.035)');horizon.addColorStop(1,'rgba(189,131,92,0)');
    ctx.fillStyle=horizon;ctx.fillRect(-300,45,600,310);
    for(const p of field){
      const z=p.z,x=(p.x-.5)*760;
      const wave=Math.sin(p.x*12+time*.18+z*7)*16+Math.cos(p.x*7-z*8+time*.12)*10;
      const y=180+z*z*180+wave*(.4+z);
      ctx.fillStyle=`rgba(201,158,122,${(.1+p.size*.42)*(.4+z*.6)})`;
      const size=(.3+p.size*.9)*(.45+z*.8);ctx.fillRect(x,y,size,size);
    }
    // A few continuous orbital traces make a quiet silhouette at reduced motion.
    ctx.strokeStyle='rgba(233,234,224,.055)';ctx.lineWidth=.6;
    ctx.beginPath();ctx.ellipse(0,0,165,172,-.17,0,Math.PI*2);ctx.stroke();
    const light=ctx.createRadialGradient(1,-164,0,1,-164,12);
    light.addColorStop(0,'rgba(255,255,246,.95)');light.addColorStop(.15,'rgba(255,255,246,.65)');light.addColorStop(1,'rgba(255,255,246,0)');
    ctx.fillStyle=light;ctx.fillRect(-15,-180,32,32);
    ctx.restore();
  }
  function tick(now){
    frame=0;
    if(paused||document.hidden||!inView)return;
    if(now-last>=32){time+=Math.min((now-last)/1000,.06);last=now;paint();}
    frame=requestAnimationFrame(tick);
  }
  function sync(){
    cancelAnimationFrame(frame);frame=0;last=performance.now();
    toggle.setAttribute('aria-pressed',String(paused));
    toggle.querySelector('.motion-label').textContent=paused?'Play motion':'Pause motion';
    toggle.querySelector('.motion-icon').textContent=paused?'▷':'Ⅱ';
    if(!paused&&!document.hidden&&inView)frame=requestAnimationFrame(tick);
  }
  function resize(){
    const rect=figure.getBoundingClientRect();width=rect.width;height=rect.height;
    const ratio=Math.min(devicePixelRatio||1,1.5);
    canvas.width=Math.round(width*ratio);canvas.height=Math.round(height*ratio);
    ctx.setTransform(ratio,0,0,ratio,0,0);paint();
  }
  toggle.addEventListener('click',()=>{paused=!paused;sync();});
  reduced.addEventListener('change',()=>{paused=reduced.matches;sync();});
  document.addEventListener('visibilitychange',sync);
  new ResizeObserver(resize).observe(figure);
  new IntersectionObserver(entries=>{inView=entries[0].isIntersecting;sync();},{threshold:.05}).observe(figure);
  figure.classList.add('scene-ready');resize();sync();
})();
