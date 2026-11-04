import test from 'node:test';
import assert from 'node:assert/strict';
import {createVisualState, advanceVisualState, orbPulseAt, qualityProfiles, initialQuality, lowerQuality} from '../visual-state.js';

test('shared pulse is bounded, continuous at cycle boundaries and periodic',()=>{
  const period=2*Math.PI/.82;
  for(let i=0;i<=10000;i++){const t=i*.1,p=orbPulseAt(t);assert.ok(p>=0&&p<=1);assert.ok(Math.abs(p-orbPulseAt(t+period))<1e-12);}
  assert.ok(Math.abs(orbPulseAt(period-1e-5)-orbPulseAt(period+1e-5))<1e-5);
});
test('pausing freezes both clock and light; resuming preserves phase',()=>{
  const state=createVisualState({x:0,y:10,z:0},'medium',false);
  advanceVisualState(state,2.5);const before={...state};state.paused=true;
  advanceVisualState(state,120);assert.equal(state.time,before.time);assert.equal(state.orbPulse,before.orbPulse);
  state.paused=false;advanceVisualState(state,1/60);assert.equal(state.time,2.5+1/60);assert.equal(state.orbPulse,orbPulseAt(state.time));
});
test('clock never moves backwards and keeps the same shared position object',()=>{
  const position={x:0,y:10,z:0},state=createVisualState(position,'low',false);
  advanceVisualState(state,-10);assert.equal(state.time,0);assert.equal(state.orbWorldPosition,position);
});
test('limited hardware, data saving and compact viewports select lower work',()=>{
  assert.equal(initialQuality({width:1920,cores:2,memory:8}),'low');
  assert.equal(initialQuality({width:1920,cores:12,memory:16,saveData:true}),'low');
  assert.equal(initialQuality({width:390,cores:12,memory:16}),'low');
  assert.equal(initialQuality({width:1440,cores:8,memory:8}),'high');
  assert.equal(initialQuality({width:768,cores:4,memory:4}),'medium');
});
test('quality degradation reduces pixels and particles without removing either field',()=>{
  for(const tier of ['high','medium']){const lower=qualityProfiles[lowerQuality(tier)],current=qualityProfiles[tier];assert.ok(lower.halo>0&&lower.tide>0);assert.ok(lower.halo<current.halo&&lower.tide<current.tide&&lower.dpr<current.dpr);}
  assert.equal(lowerQuality('low'),'low');
});
