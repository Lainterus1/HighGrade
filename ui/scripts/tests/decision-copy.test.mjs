import {test} from 'node:test';
import assert from 'node:assert/strict';
import {decisionReason} from '../../src/features/specs/decisionCopy.ts';

test('removes a complete labelled build ID and keeps useful actions',()=>{
 assert.equal(decisionReason({kind:'result',reason:'Готово. Проверенная редакция build-v1. Открыть https://localhost:18443; OAuth не проверен.'}),'Готово. Открыть https://localhost:18443; OAuth не проверен.');
});
test('preserves unrecognised labelled URLs, refs and plain text in full',()=>{
 for(const value of ['https://localhost:18443','refs/heads/main','подтверждена человеком','reviewed by author']){
  const reason=`Проверенная редакция ${value}; качество ИИ открыто.`;
  assert.equal(decisionReason({kind:'result',reason}),reason);
 }
});
