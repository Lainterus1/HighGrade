import {test, expect} from '@playwright/test';
import {api} from '../src/api/client';

test('POST StoreBusy rejects after one request; GET may retry', async () => {
 const original = globalThis.fetch;
 const calls:string[] = [];
 globalThis.fetch = async (_input, init) => {
  calls.push(init?.method ?? 'GET');
  return calls.length === 1
   ? new Response(JSON.stringify({error:{code:'StoreBusy',message:'Fixture busy'}}),{status:409})
   : new Response(JSON.stringify({saved:true}),{status:200});
 };
 try {
  await expect(api('/api/fixture',{method:'POST',body:'{}'})).rejects.toMatchObject({code:'StoreBusy',status:409});
  expect(calls).toEqual(['POST']);
  calls.length = 0;
  await expect(api('/api/fixture')).resolves.toEqual({saved:true});
  expect(calls).toEqual(['GET','GET']);
 } finally { globalThis.fetch = original; }
});

test('abort interrupts GET retry waiting', async () => {
 const original = globalThis.fetch;
 const controller = new AbortController();
 const add = controller.signal.addEventListener.bind(controller.signal);
 let waiting = false, calls = 0;
 controller.signal.addEventListener = (type, listener, options) => {
  add(type, listener, options);
  if (type === 'abort') waiting = true;
 };
 globalThis.fetch = async () => {
  calls++;
  return new Response(JSON.stringify({error:{code:'StoreBusy',message:'Fixture busy'}}),{status:409});
 };
 const pending = api('/api/fixture',{signal:controller.signal});
 const observed = pending.then(() => ({name:'UnexpectedSuccess'}), error => error);
 let deadline:ReturnType<typeof setTimeout>|undefined;
 try {
  await new Promise(resolve => setTimeout(resolve,20));
  expect(waiting).toBe(true);
  expect(calls).toBe(1);
  controller.abort();
  const result = await Promise.race([observed, new Promise(resolve => {
   deadline = setTimeout(() => resolve({name:'RetryWaitDidNotCancel'}),80);
  })]);
  expect(result).toMatchObject({name:'AbortError'});
  expect(calls).toBe(1);
 } finally {
  clearTimeout(deadline);controller.abort();await observed;globalThis.fetch = original;
 }
});
