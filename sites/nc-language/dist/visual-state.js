/** Shared animation contract. Time advances only while the scene can render. */
export const qualityProfiles = Object.freeze({
  high: {halo: 19000, tide: 34000, dpr: 1.5, targetFPS: 60},
  medium: {halo: 12000, tide: 22000, dpr: 1.25, targetFPS: 60},
  low: {halo: 7500, tide: 9500, dpr: 1, targetFPS: 30},
});

export const tideParameters = Object.freeze({
  waveAmplitude: 1.35, waveSpeed: 0.24, wavelength: 0.13,
  particleSize: 1.20, baseOpacity: 0.55, amberTint: [0.84, 0.53, 0.34],
});

export function orbPulseAt(time) { return 0.5 + 0.5 * Math.sin(time * 0.82); }
// The halo's delayed wave uses exactly the same oscillator as the orb/hand.
export const pulseGLSL = 'float orbPulseAt(float t) { return 0.5 + 0.5 * sin(t * 0.82); }';

export function initialQuality({width, cores = 4, memory = 4, saveData = false}) {
  if (saveData || cores <= 2 || memory <= 2 || width < 600) return 'low';
  return width >= 1200 && cores >= 6 && memory >= 4 ? 'high' : 'medium';
}

export function lowerQuality(tier) { return tier === 'high' ? 'medium' : 'low'; }

/** @typedef {{time:number, orbPulse:number, orbWorldPosition:import('./vendor/three/0.187.0/three.module.min.js').Vector3, qualityTier:'high'|'medium'|'low', paused:boolean}} VisualState */

export function createVisualState(position, qualityTier, paused) {
  return {time: 0, orbPulse: orbPulseAt(0), orbWorldPosition: position, qualityTier, paused};
}

export function advanceVisualState(state, elapsedSeconds) {
  if(!state.paused){state.time+=Math.max(0,elapsedSeconds);state.orbPulse=orbPulseAt(state.time);}
  return state;
}
