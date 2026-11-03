// Decorative canvases share one bounded frame loop; the page remains ordinary HTML.
(() => {
  const canvas=document.querySelector('#continuity-canvas');
  const atmosphere=document.querySelector('#page-particles');
  const toggle=document.querySelector('#motion-toggle');
  if(!canvas||!atmosphere||!toggle)return;
  const ctx=canvas.getContext('2d'),sky=atmosphere.getContext('2d');
  if(!ctx||!sky)return;
  const figure=canvas.closest('figure'),dock=toggle.closest('.motion-controls');
  const reduced=matchMedia('(prefers-reduced-motion: reduce)');
  const fine=matchMedia('(hover: hover) and (pointer: fine)');
  let savedPause=false;
  try{savedPause=localStorage.getItem('nc-motion')==='paused';}catch{}
  let paused=reduced.matches||savedPause,heroVisible=true,frame=0,last=0,time=0;
  let width=0,height=0,vw=0,vh=0,scroll=window.scrollY,compact=false;
  let rect={left:0,top:0,width:1,height:1};
  let seed=12801;
  const random=()=>{seed=(seed*1664525+1013904223)>>>0;return seed/4294967296;};
  const clamp=(value,min,max)=>Math.max(min,Math.min(max,value));
  const wrap=(value,size)=>((value%size)+size)%size;
  const pointer={x:0,y:0,nx:0,ny:0,active:false};
  const camera={x:0,y:0,light:0};
  const makeSpring=()=>({dx:0,dy:0,vx:0,vy:0});
  const ring=Array.from({length:2900},()=>({u:random()*Math.PI*2,v:random()*Math.PI*2,spread:random(),size:random(),phase:random()*Math.PI*2,...makeSpring()}));
  const dust=Array.from({length:700},()=>({x:random(),z:random(),size:random(),phase:random()*6.28}));
  const orbit=Array.from({length:85},()=>({angle:random()*Math.PI*2,r:205+random()*100,z:random()*240-120,size:random(),phase:random()*6.28}));
  const stars=Array.from({length:460},()=>({u:random(),v:random(),depth:.18+random()*.82,size:random(),phase:random()*6.28,...makeSpring()}));
  let palette={rgb:'222,227,217',warm:'212,170,129',light:false};
  function setPalette(){
    const light=document.documentElement.dataset.theme==='light';
    palette=light?{rgb:'61,77,65',warm:'130,91,60',light:true}:{rgb:'222,227,217',warm:'212,170,129',light:false};
    if(paused)paint(0);
  }
  // Repulsion is followed by a damped spring, so a pointer never leaves a permanent hole.
  function spring(point,x,y,mx,my,radius,strength,step,active){
    if(!step)return;
    if(active){
      const dx=x+point.dx-mx,dy=y+point.dy-my,distance=Math.hypot(dx,dy);
      if(distance<radius){
        const push=(1-distance/radius)**2*strength;
        const divisor=Math.max(distance,1);
        point.vx+=(dx/divisor)*push*step;
        point.vy+=(dy/divisor)*push*step;
      }
    }
    point.vx=(point.vx-point.dx*.035*step)*Math.pow(.84,step);
    point.vy=(point.vy-point.dy*.035*step)*Math.pow(.84,step);
    point.dx+=point.vx*step;point.dy+=point.vy*step;
  }
  function paintAtmosphere(step){
    sky.clearRect(0,0,vw,vh);
    const neighbors=[];
    const stride=compact?2:1;
    for(let i=0;i<stars.length;i+=stride){
      const p=stars[i],depth=p.depth;
      const x=wrap(p.u*(vw+80)+camera.x*depth*24+Math.sin(time*.06+p.phase)*8,vw+80)-40;
      const y=wrap(p.v*(vh+80)-(paused?0:scroll)*depth*.09+time*(.6+depth*1.1),vh+80)-40;
      spring(p,x,y,pointer.x,pointer.y,150,1.7,step,pointer.active&&fine.matches&&!paused);
      const px=x+p.dx,py=y+p.dy;
      const distance=pointer.active?Math.hypot(px-pointer.x,py-pointer.y):1000;
      const proximity=clamp(1-distance/175,0,1)*camera.light;
      const alpha=(.065+depth*.15+p.size*.07+proximity*.22)*(palette.light ? .72 : 1);
      const radius=.25+p.size*(.6+depth*.75);
      sky.fillStyle=`rgba(${i%6===0?palette.warm:palette.rgb},${alpha})`;
      sky.beginPath();sky.arc(px,py,radius,0,Math.PI*2);sky.fill();
      if(distance<145&&neighbors.length<10)neighbors.push({x:px,y:py});
    }
    if(camera.light>.01){
      const aura=sky.createRadialGradient(pointer.x,pointer.y,0,pointer.x,pointer.y,180);
      aura.addColorStop(0,`rgba(${palette.warm},${camera.light*(palette.light ? .025 : .045)})`);
      aura.addColorStop(1,`rgba(${palette.warm},0)`);
      sky.fillStyle=aura;sky.fillRect(pointer.x-180,pointer.y-180,360,360);
      sky.lineWidth=.6;
      for(let i=0;i<neighbors.length;i++)for(let j=i+1;j<neighbors.length;j++){
        const a=neighbors[i],b=neighbors[j],distance=Math.hypot(a.x-b.x,a.y-b.y);
        if(distance>90)continue;
        sky.strokeStyle=`rgba(${palette.rgb},${(1-distance/90)*.12*camera.light})`;
        sky.beginPath();sky.moveTo(a.x,a.y);sky.lineTo(b.x,b.y);sky.stroke();
      }
    }
  }
  function paintHero(step){
    if(!heroVisible||!width||!height)return;
    ctx.clearRect(0,0,width,height);
    const scale=Math.min(width/620,height/610),cx=width*.51,cy=height*.43;
    const mx=(pointer.x-rect.left-cx)/scale,my=(pointer.y-rect.top-cy)/scale;
    const inside=pointer.active&&pointer.x>rect.left&&pointer.x<rect.left+width&&pointer.y>rect.top&&pointer.y<rect.top+height;
    const tiltX=-.28+camera.y*.25+Math.sin(time*.1)*.06;
    const tiltY=.31+camera.x*.43+Math.sin(time*.08)*.15;
    const cosX=Math.cos(tiltX),sinX=Math.sin(tiltX),cosY=Math.cos(tiltY),sinY=Math.sin(tiltY);
    ctx.save();ctx.translate(cx,cy);ctx.scale(scale,scale);
    const haze=ctx.createRadialGradient(-20,15,10,0,20,285);
    haze.addColorStop(0,'rgba(229,235,220,.018)');haze.addColorStop(.5,'rgba(203,221,207,.026)');haze.addColorStop(1,'rgba(200,220,204,0)');
    ctx.fillStyle=haze;ctx.fillRect(-350,-290,700,650);
    // Fainter orbits sit behind the volume rather than flattening it into one circle.
    ctx.lineWidth=.6;ctx.strokeStyle='rgba(221,228,212,.045)';
    ctx.beginPath();ctx.ellipse(0,5,217,67,-.32+camera.x*.09,0,Math.PI*2);ctx.stroke();
    ctx.beginPath();ctx.ellipse(0,5,209,91,.72+camera.y*.09,0,Math.PI*2);ctx.stroke();
    for(const p of orbit){
      const angle=p.angle+time*.025,projection=650/(650-p.z);
      const x=Math.cos(angle)*p.r*projection,y=Math.sin(angle)*p.r*.82*projection;
      const radius=.25+p.size*1.4*projection;
      ctx.fillStyle=`rgba(214,222,205,${.05+p.size*.18})`;
      ctx.beginPath();ctx.arc(x,y,radius,0,Math.PI*2);ctx.fill();
    }
    const projected=[];
    for(let i=0;i<ring.length;i+=(compact?2:1)){
      const p=ring[i],u=p.u+time*.018;
      const tube=33+(p.spread-.5)*14,radius=156+Math.cos(p.v)*tube;
      const x=Math.cos(u)*radius,y=Math.sin(u)*radius,z=Math.sin(p.v)*tube;
      const xx=x*cosY+z*sinY,zz=-x*sinY+z*cosY;
      const yy=y*cosX-zz*sinX,depth=y*sinX+zz*cosX;
      const projection=650/(650-depth),px=xx*projection,py=yy*projection;
      spring(p,px,py,mx,my,105,2.15,step,inside&&fine.matches&&!paused);
      const front=clamp((depth+120)/240,0,1);
      projected.push({x:px+p.dx,y:py+p.dy,z:depth,r:(.3+p.size*.95)*projection,alpha:(.1+p.size*.52)*(.55+front*.8),front});
    }
    projected.sort((a,b)=>a.z-b.z);
    for(const p of projected){
      ctx.fillStyle=`rgba(235,238,224,${p.alpha})`;
      ctx.beginPath();ctx.arc(p.x,p.y,p.r,0,Math.PI*2);ctx.fill();
      if(p.front>.68&&p.r>1.1){ctx.fillStyle='rgba(235,239,225,.035)';ctx.beginPath();ctx.arc(p.x,p.y,p.r*2.5,0,Math.PI*2);ctx.fill();}
    }
    const horizon=ctx.createRadialGradient(8,188,1,8,188,180);
    horizon.addColorStop(0,'rgba(255,230,187,.35)');horizon.addColorStop(.12,'rgba(223,174,124,.11)');horizon.addColorStop(.5,'rgba(179,129,86,.025)');horizon.addColorStop(1,'rgba(179,129,86,0)');
    ctx.fillStyle=horizon;ctx.fillRect(-300,20,600,410);
    for(const p of dust){
      const depth=p.z,x=(p.x-.5)*800+camera.x*depth*18;
      const wave=Math.sin(p.x*12+time*.18+depth*7)*15+Math.cos(p.x*7-depth*8+time*.12)*9;
      const y=183+depth*depth*180+wave*(.4+depth)+camera.y*depth*8;
      ctx.fillStyle=`rgba(207,165,126,${(.08+p.size*.42)*(.4+depth*.6)})`;
      const size=(.3+p.size*.9)*(.4+depth*.85);ctx.fillRect(x,y,size,size);
    }
    const lx=-24+camera.x*18,ly=-163+camera.y*10;
    const light=ctx.createRadialGradient(lx,ly,0,lx,ly,17);
    light.addColorStop(0,'rgba(255,255,241,.95)');light.addColorStop(.12,'rgba(255,255,241,.48)');light.addColorStop(1,'rgba(255,255,241,0)');
    ctx.fillStyle=light;ctx.fillRect(lx-18,ly-18,36,36);
    ctx.restore();
    figure.style.setProperty('--scene-shift-x',`${(camera.x*9).toFixed(2)}px`);
    figure.style.setProperty('--scene-shift-y',`${(camera.y*7).toFixed(2)}px`);
    figure.style.setProperty('--scene-turn-x',`${(-camera.y*7).toFixed(2)}deg`);
    figure.style.setProperty('--scene-turn-y',`${(camera.x*8).toFixed(2)}deg`);
  }
  function paint(step){
    if(step){
      const ease=1-Math.pow(.91,step);
      camera.x+=(pointer.nx-camera.x)*ease;camera.y+=(pointer.ny-camera.y)*ease;
      camera.light+=((pointer.active?1:0)-camera.light)*ease;
    }
    paintAtmosphere(step);paintHero(step);
  }
  function tick(now){
    frame=0;if(paused||document.hidden)return;
    const elapsed=now-last;
    if(elapsed>=32){const seconds=Math.min(elapsed/1000,.065);last=now;time+=seconds;paint(seconds*30);}
    frame=requestAnimationFrame(tick);
  }
  function sync(){
    cancelAnimationFrame(frame);frame=0;last=performance.now();
    toggle.setAttribute('aria-pressed',String(paused));
    toggle.querySelector('.motion-label').textContent=paused?'Play motion':'Pause motion';
    toggle.querySelector('.motion-icon').textContent=paused?'▷':'Ⅱ';
    if(!paused&&!document.hidden)frame=requestAnimationFrame(tick);
  }
  function measureHero(){rect=figure.getBoundingClientRect();}
  function resize(){
    vw=document.documentElement.clientWidth;vh=window.innerHeight;
    compact=vw<700||!fine.matches;
    const ratio=Math.min(devicePixelRatio||1,1.5);
    atmosphere.width=Math.round(vw*ratio);atmosphere.height=Math.round(vh*ratio);
    sky.setTransform(ratio,0,0,ratio,0,0);
    measureHero();width=rect.width;height=rect.height;
    canvas.width=Math.round(width*ratio);canvas.height=Math.round(height*ratio);
    ctx.setTransform(ratio,0,0,ratio,0,0);paint(0);
  }
  function leave(){pointer.active=false;pointer.nx=0;pointer.ny=0;}
  window.addEventListener('pointermove',event=>{
    if(!fine.matches||event.pointerType!=='mouse'||paused)return;
    pointer.x=event.clientX;pointer.y=event.clientY;
    pointer.nx=clamp((event.clientX/vw-.5)*2,-1,1);
    pointer.ny=clamp((event.clientY/vh-.5)*2,-1,1);pointer.active=true;
  },{passive:true});
  document.documentElement.addEventListener('pointerleave',leave);
  window.addEventListener('blur',leave);
  window.addEventListener('scroll',()=>{scroll=window.scrollY;measureHero();if(paused)paint(0);},{passive:true});
  window.addEventListener('resize',resize,{passive:true});
  toggle.addEventListener('click',()=>{
    paused=!paused;if(paused)leave();
    try{localStorage.setItem('nc-motion',paused?'paused':'playing');}catch{}
    sync();
  });
  reduced.addEventListener('change',()=>{paused=reduced.matches;leave();paint(0);sync();});
  fine.addEventListener('change',()=>{leave();resize();});
  document.addEventListener('visibilitychange',()=>{if(document.hidden)leave();sync();});
  new ResizeObserver(resize).observe(figure);
  new IntersectionObserver(entries=>{heroVisible=entries[0].isIntersecting;if(heroVisible)paint(0);},{threshold:.01}).observe(figure);
  new MutationObserver(setPalette).observe(document.documentElement,{attributes:true,attributeFilter:['data-theme']});
  document.body.classList.add('has-atmosphere');figure.classList.add('scene-ready');dock.hidden=false;
  setPalette();resize();sync();
})();
