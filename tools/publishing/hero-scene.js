import * as THREE from './vendor/three/0.187.0/three.module.min.js';
import {qualityProfiles, tideParameters, createVisualState, advanceVisualState, orbPulseAt, initialQuality, lowerQuality} from './visual-state.js';
import {haloVertex, haloFragment, tideVertex, tideFragment, orbVertex, orbFragment, glowVertex, glowFragment, bloomVertex, bloomFragment} from './hero-shaders.js';

const stage = document.querySelector('.cinematic-hero');
const canvas = document.querySelector('#continuity-canvas');
const visual = canvas?.closest('figure');
const diagnostic = document.querySelector('#scene-status');
const params = new URLSearchParams(location.search);
const finePointer = matchMedia('(hover:hover) and (pointer:fine)');
const reduced = matchMedia('(prefers-reduced-motion:reduce)');

function observeVitals() {
  if (!('PerformanceObserver' in window)) return;
  let cls = 0;
  for (const type of ['largest-contentful-paint', 'layout-shift', 'event']) {
    if (!PerformanceObserver.supportedEntryTypes.includes(type)) continue;
    const observer = new PerformanceObserver(list => {
      for (const entry of list.getEntries()) {
        if (type === 'largest-contentful-paint') diagnostic.dataset.lcpMs = String(Math.round(entry.startTime));
        if (type === 'layout-shift' && !entry.hadRecentInput) {cls += entry.value; diagnostic.dataset.cls = cls.toFixed(4);}
        if (type === 'event') diagnostic.dataset.maxEventMs = String(Math.max(Number(diagnostic.dataset.maxEventMs || 0), entry.duration));
      }
    });
    observer.observe({type, buffered:true, ...(type === 'event' ? {durationThreshold:16} : {})});
    addEventListener('pagehide', () => observer.disconnect(), {once:true});
  }
}

function initialize() {
  observeVitals();
  if (params.get('visual') === 'fallback') {diagnostic.dataset.renderer = 'fallback'; return;}
  const context = canvas.getContext('webgl2', {alpha:true, antialias:false, powerPreference:'low-power'});
  if (!context) {diagnostic.dataset.renderer = 'fallback'; return;}
  const renderer = new THREE.WebGLRenderer({canvas, context, alpha:true, antialias:false});
  renderer.autoClear = false;
  renderer.setClearColor(0x000000, 0);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  diagnostic.dataset.bloom='off';
  const gpuTimer=context.getExtension('EXT_disjoint_timer_query_webgl2');
  let gpuQuery=null,gpuQueryTier='';
  const scene = new THREE.Scene(), seaScene = new THREE.Scene();
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 2000);
  camera.position.z = 1000;
  const seaCamera = new THREE.PerspectiveCamera(54, 1, 0.1, 180);
  seaCamera.position.set(0, 7, 22);
  seaCamera.lookAt(0, -1, -20);
  const forcedQuality=['high','medium','low'].includes(params.get('quality'));
  const tier = forcedQuality ? params.get('quality') : initialQuality({
    width:innerWidth, cores:navigator.hardwareConcurrency, memory:navigator.deviceMemory,
    saveData:navigator.connection?.saveData,
  });
  const state = createVisualState(new THREE.Vector3(), tier, document.documentElement.dataset.motion ? document.documentElement.dataset.motion === 'paused' : reduced.matches);
  const uniforms = {
    uTime:{value:0}, uOrbPulse:{value:state.orbPulse}, uOrbPosition:{value:state.orbWorldPosition},
    uResolution:{value:new THREE.Vector2()}, uPixelRatio:{value:1}, uRadius:{value:1}, uHaloY:{value:0},
    uPointer:{value:new THREE.Vector2()}, uWaveAmplitude:{value:tideParameters.waveAmplitude},
    uWaveSpeed:{value:tideParameters.waveSpeed}, uWavelength:{value:tideParameters.wavelength},
    uParticleSize:{value:tideParameters.particleSize}, uBaseOpacity:{value:tideParameters.baseOpacity},
    uAmber:{value:new THREE.Vector3(...tideParameters.amberTint)},
  };
  const material = (vertexShader, fragmentShader, extra={}) => new THREE.ShaderMaterial({
    uniforms, vertexShader, fragmentShader, transparent:true, depthWrite:false,
    blending:THREE.AdditiveBlending, ...extra,
  });
  let seed = 19331;
  const random = () => {seed=(seed*1664525+1013904223)>>>0;return seed/4294967296;};
  function geometry(count, kind) {
    const points = new Float32Array(count*3), seeds = new Float32Array(count);
    const columns = Math.ceil(Math.sqrt(count*1.7)), rows = Math.ceil(count/columns);
    for(let i=0;i<count;i++) {
      const offset=i*3;
      if(kind==='halo') {
        points[offset]=random()*Math.PI*2;
        // A feathered annulus, irregular density and depth, never an exact torus.
        points[offset+1]=0.88+(random()+random()+random())/3*0.27;
        points[offset+2]=(random()-.5)*.19;
      } else {
        points[offset]=((i%columns+random()*.8)/columns-.5)*150;
        points[offset+1]=0;
        points[offset+2]=((Math.floor(i/columns)+random()*.8)/rows)*100-77;
      }
      seeds[i]=random();
    }
    // Every prefix still covers the whole field when the quality drops.
    for(let i=count-1;i>0;i--){const j=Math.floor(random()*(i+1));for(let k=0;k<3;k++){const a=i*3+k,b=j*3+k,temp=points[a];points[a]=points[b];points[b]=temp;}const s=seeds[i];seeds[i]=seeds[j];seeds[j]=s;}
    const geometry=new THREE.BufferGeometry();
    geometry.setAttribute('position',new THREE.BufferAttribute(points,3));
    geometry.setAttribute('aSeed',new THREE.BufferAttribute(seeds,1));
    geometry.setDrawRange(0,count);
    return geometry;
  }
  const maximum=qualityProfiles[tier];
  const halo=new THREE.Points(geometry(maximum.halo,'halo'),material(haloVertex,haloFragment));
  const sea=new THREE.Points(geometry(maximum.tide,'tide'),material(tideVertex,tideFragment));
  halo.frustumCulled=sea.frustumCulled=false;
  scene.add(halo); seaScene.add(sea);
  const orb=new THREE.Mesh(new THREE.SphereGeometry(1,64,48),material(orbVertex,orbFragment,{blending:THREE.NormalBlending,depthTest:false}));
  const glow=new THREE.Mesh(new THREE.PlaneGeometry(1,1),material(glowVertex,glowFragment,{depthTest:false}));
  glow.renderOrder=1; orb.renderOrder=2; scene.add(glow,orb);
  const orbitPoints=Array.from({length:193},(_,i)=>new THREE.Vector3(Math.cos(i/192*Math.PI),Math.sin(i/192*Math.PI),0));
  const orbit=new THREE.Line(new THREE.BufferGeometry().setFromPoints(orbitPoints),new THREE.LineBasicMaterial({color:0xd5cec3,transparent:true,opacity:.12,depthWrite:false}));
  const satellite=new THREE.Mesh(new THREE.SphereGeometry(1,12,8),new THREE.MeshBasicMaterial({color:0xf7f2e8}));
  scene.add(orbit,satellite);
  // Half-resolution, thresholded light pass. DOM text and the photograph never enter it.
  let bloom=null,bloomActive=false,bloomTried=false;
  const postScene=new THREE.Scene(),postCamera=new THREE.Camera();
  const postUniforms={uInput:{value:null},uDirection:{value:new THREE.Vector2()},uExtract:{value:1},uStrength:{value:1}};
  const postMaterial=new THREE.ShaderMaterial({uniforms:postUniforms,vertexShader:bloomVertex,fragmentShader:bloomFragment,transparent:true,depthWrite:false,depthTest:false});
  const postQuad=new THREE.Mesh(new THREE.PlaneGeometry(2,2),postMaterial);postQuad.frustumCulled=false;postScene.add(postQuad);
  function resizeBloom(){
    if(!bloom)return;
    const ratio=renderer.getPixelRatio();bloom.width=Math.max(1,Math.round(width*ratio*.5));bloom.height=Math.max(1,Math.round(height*ratio*.5));
    bloom.light.setSize(bloom.width,bloom.height);bloom.blur.setSize(bloom.width,bloom.height);
  }
  function activateBloom(){
    bloomTried=true;bloomActive=true;
    const options={depthBuffer:false,stencilBuffer:false,minFilter:THREE.LinearFilter,magFilter:THREE.LinearFilter};
    bloom={light:new THREE.WebGLRenderTarget(1,1,options),blur:new THREE.WebGLRenderTarget(1,1,options),width:1,height:1};
    resizeBloom();delete diagnostic.dataset.gpuTier;diagnostic.dataset.bloom='selective';
  }
  let width=0,height=0,seaHeight=0,orbRadius=1,visible=true,lost=false,disposed=false;
  let frame=0,lastPaint=0,sampleStart=0,sampleFrames=0,slowWindows=0;
  const pointer={x:0,y:0}, eased={x:0,y:0};

  function setQuality(next) {
    state.qualityTier=next;
    const profile=qualityProfiles[next];
    halo.geometry.setDrawRange(0,Math.min(maximum.halo,profile.halo));
    sea.geometry.setDrawRange(0,Math.min(maximum.tide,profile.tide));
    const ratio=Math.min(devicePixelRatio||1,profile.dpr);
    renderer.setPixelRatio(ratio); uniforms.uPixelRatio.value=ratio;
    diagnostic.dataset.quality=next;
    if(next!=='high'&&bloomActive){bloomActive=false;diagnostic.dataset.bloom='off';}
    diagnostic.dataset.particles=String(halo.geometry.drawRange.count+sea.geometry.drawRange.count);
    if(width) renderer.setSize(width,height,false);
  }
  function resize() {
    const box=visual.getBoundingClientRect(); width=box.width; height=box.height;
    seaHeight=Math.round(height*(width<600?.36:.41));
    camera.left=-width/2;camera.right=width/2;camera.top=height/2;camera.bottom=-height/2;camera.updateProjectionMatrix();
    seaCamera.aspect=width/seaHeight;seaCamera.updateProjectionMatrix();
    renderer.setSize(width,height,false);uniforms.uResolution.value.set(width,height);
    resizeBloom();
    const compact=width<600;
    // Coordinates belong to the figure, independently of the surrounding copy.
    orbRadius=Math.min(85,Math.max(55,width*.11));
    state.orbWorldPosition.set(0,height*.295,0);
    uniforms.uRadius.value=Math.min(width*.42,height*.38);
    uniforms.uHaloY.value=height*.05;
    orbit.scale.setScalar(uniforms.uRadius.value*1.08);orbit.position.y=uniforms.uHaloY.value;
    satellite.scale.setScalar(compact?2.2:3.1);
    if(state.paused||!visible) paint();
  }
  function publishState(now) {
    diagnostic.dataset.time=state.time.toFixed(3);
    diagnostic.dataset.orbPulse=state.orbPulse.toFixed(4);
    diagnostic.dataset.paused=String(state.paused);
    diagnostic.dataset.visible=String(visible);
    if(sampleStart&&now-sampleStart>=2400&&sampleFrames) {
      const fps=sampleFrames*1000/(now-sampleStart);
      diagnostic.dataset.fps=fps.toFixed(1);
      const threshold=state.qualityTier==='high'?43:32;
      slowWindows=fps<threshold?slowWindows+1:0;
      if(!forcedQuality&&slowWindows>=2&&state.qualityTier!=='low') {setQuality(lowerQuality(state.qualityTier));slowWindows=0;}
      sampleStart=now;sampleFrames=0;
    }
    if(!sampleStart)sampleStart=now;
  }
  function paint(now=performance.now()) {
    if(lost||disposed)return;
    const cpuStart=performance.now();
    state.orbPulse=orbPulseAt(state.time);
    uniforms.uTime.value=state.time;uniforms.uOrbPulse.value=state.orbPulse;
    eased.x+=(pointer.x-eased.x)*.045;eased.y+=(pointer.y-eased.y)*.045;
    uniforms.uPointer.value.set(state.paused?0:eased.x,state.paused?0:eased.y);
    const float=state.paused?0:Math.sin(state.time*.41)*2.1;
    orb.position.copy(state.orbWorldPosition);orb.position.y+=float;
    orb.scale.setScalar(orbRadius*(1+state.orbPulse*.007));
    glow.position.copy(orb.position);glow.scale.set(orbRadius*5.6,orbRadius*5.6,1);
    const angle=Math.PI*.5+Math.sin(state.time*.05)*.06;
    satellite.position.set(Math.cos(angle)*orbit.scale.x,Math.sin(angle)*orbit.scale.y+orbit.position.y,0);
    stage.style.setProperty('--orb-pulse',state.orbPulse.toFixed(4));
    stage.style.setProperty('--hand-shift-x',`${state.paused?0:eased.x*3}px`);
    stage.style.setProperty('--hand-shift-y',`${state.paused?0:eased.y*2}px`);
    if(gpuQuery&&context.getQueryParameter(gpuQuery,context.QUERY_RESULT_AVAILABLE)){
      if(!context.getParameter(gpuTimer.GPU_DISJOINT_EXT)){diagnostic.dataset.gpuMs=(context.getQueryParameter(gpuQuery,context.QUERY_RESULT)/1e6).toFixed(2);diagnostic.dataset.gpuTier=gpuQueryTier;}
      context.deleteQuery(gpuQuery);gpuQuery=null;
    }
    if(!bloomTried&&state.qualityTier==='high'&&diagnostic.dataset.gpuTier==='high'&&Number(diagnostic.dataset.gpuMs)<8)activateBloom();
    if(bloomActive&&diagnostic.dataset.gpuTier==='high'&&Number(diagnostic.dataset.gpuMs)>12){bloomActive=false;diagnostic.dataset.bloom='off-budget';}
    const measureGPU=gpuTimer&&!gpuQuery&&diagnostic.dataset.gpuTier!==state.qualityTier;
    if(measureGPU){gpuQuery=context.createQuery();gpuQueryTier=state.qualityTier;context.beginQuery(gpuTimer.TIME_ELAPSED_EXT,gpuQuery);}
    renderer.info.reset(); renderer.setScissorTest(false);renderer.setViewport(0,0,width,height);renderer.clear();
    renderer.setScissor(0,0,width,seaHeight);renderer.setViewport(0,0,width,seaHeight);renderer.setScissorTest(true);
    renderer.render(seaScene,seaCamera);
    const seaCalls=renderer.info.render.calls;
    renderer.setScissorTest(false);renderer.setViewport(0,0,width,height);renderer.render(scene,camera);
    let calls=seaCalls+renderer.info.render.calls;
    if(bloomActive){
      const ratio=uniforms.uPixelRatio.value;
      renderer.setRenderTarget(bloom.light);renderer.clear();uniforms.uPixelRatio.value=ratio*.5;
      renderer.render(scene,camera);calls+=renderer.info.render.calls;uniforms.uPixelRatio.value=ratio;
      postUniforms.uInput.value=bloom.light.texture;postUniforms.uDirection.value.set(1.65/bloom.width,0);postUniforms.uExtract.value=1;postUniforms.uStrength.value=1;
      postMaterial.blending=THREE.NoBlending;renderer.setRenderTarget(bloom.blur);renderer.clear();renderer.render(postScene,postCamera);calls++;
      postUniforms.uInput.value=bloom.blur.texture;postUniforms.uDirection.value.set(0,1.65/bloom.height);postUniforms.uExtract.value=0;postUniforms.uStrength.value=.22+state.orbPulse*.10;
      postMaterial.blending=THREE.AdditiveBlending;renderer.setRenderTarget(null);renderer.render(postScene,postCamera);calls++;
    }
    if(measureGPU)context.endQuery(gpuTimer.TIME_ELAPSED_EXT);
    diagnostic.dataset.drawCalls=String(calls);
    diagnostic.dataset.time=state.time.toFixed(3);diagnostic.dataset.orbPulse=state.orbPulse.toFixed(4);
    diagnostic.dataset.paused=String(state.paused);diagnostic.dataset.visible=String(visible);
    diagnostic.dataset.cpuSubmitMs=(performance.now()-cpuStart).toFixed(2);
  }
  function tick(now) {
    frame=0;
    if(state.paused||!visible||document.hidden||lost||disposed)return;
    const interval=1000/qualityProfiles[state.qualityTier].targetFPS;
    if(lastPaint&&now-lastPaint<interval-1) {frame=requestAnimationFrame(tick);return;}
    const delta=lastPaint?Math.min((now-lastPaint)/1000,1):0;
    lastPaint=now;advanceVisualState(state,delta);
    paint(now);sampleFrames++;publishState(now);frame=requestAnimationFrame(tick);
  }
  function reconcile() {
    cancelAnimationFrame(frame);frame=0;lastPaint=0;sampleStart=0;sampleFrames=0;
    if(!visible||document.hidden||lost||disposed) {diagnostic.dataset.visible='false';return;}
    paint();if(!state.paused)frame=requestAnimationFrame(tick);
  }
  const resizeObserver=new ResizeObserver(resize);resizeObserver.observe(visual);
  const visibilityObserver=new IntersectionObserver(entries=>{visible=entries[0].isIntersecting;reconcile();},{threshold:0.01});
  visibilityObserver.observe(stage);
  const abort=new AbortController(),options={signal:abort.signal};
  stage.addEventListener('pointermove',event=>{
    if(!finePointer.matches||state.paused)return;
    const box=visual.getBoundingClientRect();pointer.x=Math.max(-1,Math.min(1,(event.clientX-box.left)/box.width*2-1));pointer.y=Math.max(-1,Math.min(1,(event.clientY-box.top)/box.height*2-1));
  },{...options,passive:true});
  stage.addEventListener('pointerleave',()=>{pointer.x=pointer.y=0;},options);
  addEventListener('nc:motion',event=>{state.paused=event.detail.paused;reconcile();},options);
  document.addEventListener('visibilitychange',reconcile,options);
  addEventListener('pageshow',reconcile,options);
  canvas.addEventListener('webglcontextlost',event=>{event.preventDefault();lost=true;gpuQuery=null;delete diagnostic.dataset.gpuMs;delete diagnostic.dataset.gpuTier;reconcile();stage.classList.remove('gpu-ready');diagnostic.dataset.renderer='context-lost';},options);
  canvas.addEventListener('webglcontextrestored',()=>{lost=false;diagnostic.dataset.renderer='webgl2';stage.classList.add('gpu-ready');resize();reconcile();},options);
  addEventListener('pagehide',event=>{
    cancelAnimationFrame(frame);
    if(event.persisted)return;
    disposed=true;resizeObserver.disconnect();visibilityObserver.disconnect();abort.abort();
    for(const object of [halo,sea,orb,glow,orbit,satellite]){object.geometry.dispose();object.material.dispose();}
    postQuad.geometry.dispose();postMaterial.dispose();if(bloom){bloom.light.dispose();bloom.blur.dispose();}
    if(gpuQuery)context.deleteQuery(gpuQuery);renderer.dispose();
  },options);
  setQuality(tier);resize();paint();stage.classList.add('gpu-ready');diagnostic.dataset.renderer='webgl2';reconcile();
}

if(stage&&canvas&&diagnostic) {
  try {initialize();} catch(error) {
    diagnostic.dataset.renderer='fallback';stage.classList.remove('gpu-ready');
    console.warn('nC scene unavailable. The accessible landing remains available.',error);
  }
}
