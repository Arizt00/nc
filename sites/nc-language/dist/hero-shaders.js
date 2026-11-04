import {pulseGLSL} from './visual-state.js?v=1717859e3622';

export const haloVertex = /* glsl */`
  uniform float uTime, uOrbPulse, uRadius, uPixelRatio, uHaloY;
  uniform vec2 uResolution, uPointer;
  uniform vec3 uOrbPosition;
  attribute float aSeed;
  varying float vAlpha, vWarm;
  ${pulseGLSL}
  void main() {
    float angle = position.x + 0.013 * uTime + 0.018 * sin(uTime * 0.13 + aSeed * 20.0);
    float radius = uRadius * position.y;
    vec3 p = vec3(cos(angle) * radius, sin(angle) * radius, position.z * uRadius);
    p.xy += uPointer * (3.0 + position.z * 4.0);
    p.y += uHaloY;
    float distanceFromOrb = length(p.xy - uOrbPosition.xy);
    float delayed = orbPulseAt(uTime - distanceFromOrb / 180.0);
    float breath = mix(uOrbPulse, delayed, 0.7);
    float lowerFade = smoothstep(-uResolution.y * 0.035, uResolution.y * 0.14, p.y);
    float patches = 0.58 + 0.42 * sin(angle * 2.3 + 0.7) * sin(angle * 4.7 - 1.4);
    vAlpha = (0.23 + 0.64 * aSeed) * (0.66 + breath * 0.50) * lowerFade * patches * (0.72 + 0.5 * abs(cos(angle)));
    vWarm = (1.0 - smoothstep(0.0, uRadius * 1.25, distanceFromOrb)) * 0.30;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(p, 1.0);
    gl_PointSize = clamp((0.55 + aSeed * 1.25 + abs(position.z) * 0.9) * uPixelRatio, 0.7, 3.5);
  }
`;

export const haloFragment = /* glsl */`
  varying float vAlpha, vWarm;
  void main() {
    float r = length(gl_PointCoord - 0.5) * 2.0;
    float alpha = (1.0 - smoothstep(0.10, 1.0, r)) * vAlpha;
    if(alpha < 0.008) discard;
    gl_FragColor = vec4(mix(vec3(0.88, 0.87, 0.83), vec3(0.88, 0.63, 0.42), vWarm), alpha);
  }
`;

export const tideVertex = /* glsl */`
  uniform float uTime, uOrbPulse, uPixelRatio, uWaveAmplitude, uWaveSpeed, uWavelength, uParticleSize;
  uniform vec2 uPointer;
  attribute float aSeed;
  varying float vAlpha, vDepth, vCrest, vBlur;
  void main() {
    vec3 p = position;
    float t = uTime * uWaveSpeed;
    // A continuous, non-repeating sum of travelling waves. Attributes never change.
    float broad = sin(p.x * uWavelength + p.z * 0.075 - t);
    float cross = sin(p.z * 0.18 + p.x * 0.047 + t * 0.67);
    float detail = sin(p.x * 0.29 - p.z * 0.11 + t * 0.43);
    float swell = (broad * 0.67 + cross * 0.35 + detail * 0.12) * uWaveAmplitude;
    p.y = swell + 0.0017 * p.x * p.x - 1.0;
    p.x += sin(p.z * 0.035 + t * 0.21) * 0.35 + uPointer.x * 0.12;
    p.y += uPointer.y * 0.06;
    vec4 view = modelViewMatrix * vec4(p, 1.0);
    gl_Position = projectionMatrix * view;
    float depth = clamp((-view.z - 8.0) / 95.0, 0.0, 1.0);
    float near = 1.0 - depth;
    vDepth = depth;
    vCrest = smoothstep(-0.4, 1.0, swell);
    vAlpha = (0.14 + 0.64 * vCrest) * (0.55 + aSeed * 0.45) * (0.91 + uOrbPulse * 0.09);
    vAlpha *= smoothstep(0.0, 0.12, depth) * (1.0 - smoothstep(0.80, 1.0, depth));
    vBlur = smoothstep(0.45, 0.98, near);
    gl_PointSize = clamp((uParticleSize + aSeed * 1.4) * (0.4 + near * near * 3.4) * uPixelRatio, 0.65, 11.0 * uPixelRatio);
  }
`;

export const tideFragment = /* glsl */`
  uniform float uBaseOpacity;
  uniform vec3 uAmber;
  varying float vAlpha, vDepth, vCrest, vBlur;
  void main() {
    float radius = length(gl_PointCoord - 0.5) * 2.0;
    float soft = exp(-radius * radius * mix(6.0, 2.4, vBlur));
    soft *= 1.0 - smoothstep(0.82, 1.0, radius);
    float alpha = soft * vAlpha * uBaseOpacity * mix(1.0, 0.38, vBlur);
    if(alpha < 0.004) discard;
    vec3 color = mix(uAmber, vec3(1.0, 0.82, 0.62), vCrest * 0.56);
    gl_FragColor = vec4(color, alpha);
  }
`;

export const orbVertex = /* glsl */`
  varying vec3 vNormal;
  void main() { vNormal = normalize(normalMatrix * normal); gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }
`;
export const orbFragment = /* glsl */`
  uniform float uOrbPulse;
  varying vec3 vNormal;
  void main() {
    vec3 n = normalize(vNormal);
    float height = n.y * 0.5 + 0.5;
    vec3 shade = mix(vec3(0.80, 0.82, 0.82), vec3(1.075, 1.07, 1.055), smoothstep(0.04, 0.86, height));
    shade += pow(max(n.z, 0.0), 4.0) * 0.05;
    float edge = smoothstep(0.0, 0.38, n.z);
    gl_FragColor = vec4(shade * (0.96 + uOrbPulse * 0.06), edge);
  }
`;
export const glowVertex = /* glsl */`
  varying vec2 vUv;
  void main(){ vUv=uv; gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0); }
`;
export const glowFragment = /* glsl */`
  uniform float uOrbPulse;
  varying vec2 vUv;
  void main(){
    float radius=length(vUv-0.5)*2.0;
    float halo=exp(-radius*radius*5.5)*(1.0-smoothstep(0.65,1.0,radius));
    float softRim=exp(-pow((radius-0.355)*7.0,2.0));
    gl_FragColor=vec4(vec3(0.98,0.965,0.94),softRim*(0.23+uOrbPulse*0.16)+halo*0.10);
  }
`;

export const bloomVertex = /* glsl */`
  varying vec2 vUv;
  void main(){vUv=uv;gl_Position=vec4(position.xy,0.0,1.0);}
`;
export const bloomFragment = /* glsl */`
  uniform sampler2D uInput;
  uniform vec2 uDirection;
  uniform float uExtract, uStrength;
  varying vec2 vUv;
  vec3 lightAt(vec2 uv){
    vec3 color=texture2D(uInput,uv).rgb;
    return mix(color,max(color-vec3(0.72),vec3(0.0))/0.28,uExtract);
  }
  void main(){
    vec3 color=lightAt(vUv)*0.227027;
    color+=(lightAt(vUv+uDirection*1.384615)+lightAt(vUv-uDirection*1.384615))*0.316216;
    color+=(lightAt(vUv+uDirection*3.230769)+lightAt(vUv-uDirection*3.230769))*0.070270;
    gl_FragColor=vec4(color,uStrength);
  }
`;
