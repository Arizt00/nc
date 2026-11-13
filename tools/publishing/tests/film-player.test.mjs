import {test} from 'node:test';
import assert from 'node:assert/strict';
import {resolveObjectURL} from 'node:buffer';
import {prepareFilmVideo} from '../film-player.js';

const video = () => ({src:'unchanged',querySelector:() => ({src:'https://ncc-lenguage.org/assets/media/film.mp4'})});

test('Playback uses the complete original response in a seekable local blob', async t => {
  const bytes = new Uint8Array([0,0,0,32,102,116,121,112]);
  const request = t.mock.method(globalThis,'fetch',async () => new Response(bytes,{headers:{'Content-Type':'video/mp4'}}));
  const element = video();
  const url = await prepareFilmVideo(element);
  try {
    assert.equal(request.mock.calls.length,1);
    assert.equal(request.mock.calls[0].arguments[0],'https://ncc-lenguage.org/assets/media/film.mp4');
    assert.equal(element.src,url);
    assert.match(url,/^blob:/);
    const blob = resolveObjectURL(url);
    assert.equal(blob.type,'video/mp4');
    assert.deepEqual(new Uint8Array(await blob.arrayBuffer()),bytes);
  } finally { URL.revokeObjectURL(url); }
});

test('A failed request preserves the native source fallback', async t => {
  t.mock.method(globalThis,'fetch',async () => new Response('',{status:503}));
  const element = video();
  await assert.rejects(prepareFilmVideo(element),/request failed/);
  assert.equal(element.src,'unchanged');
});

test('An HTML fallback or empty body is never installed as a video source', async t => {
  const request = t.mock.method(globalThis,'fetch',async () => new Response('<html>Not found</html>',{headers:{'Content-Type':'text/html'}}));
  const element = video();
  await assert.rejects(prepareFilmVideo(element),/Invalid video response/);
  request.mock.mockImplementation(async () => new Response('',{headers:{'Content-Type':'video/mp4'}}));
  await assert.rejects(prepareFilmVideo(element),/Invalid video response/);
  assert.equal(element.src,'unchanged');
});
