import type {Handoff} from '@/api/client';

// This generated handoff instruction duplicates the controls and exposes a build-time path.
const deliveryInstruction='Результат готов: проверьте интерфейс и примите либо верните на доработку. Проверки и версия — docs/evidence/specification-ui/handoff.md.';
export function decisionReason(request:Handoff){
 return request.kind==='result'&&request.reason.trim()===deliveryInstruction?'':request.reason;
}
