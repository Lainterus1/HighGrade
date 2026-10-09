import type {Handoff} from '@/api/client';
import {decisionReason} from './decisionCopy';

export function DecisionReason({request}:{request:Handoff}){
 const text=decisionReason(request);
 return <>{text&&<p className="source-text">{text}</p>}{request.kind==='result'&&text!==request.reason.trim()&&<details className="result-source-details"><summary>Исходные сведения о результате</summary><p className="source-text">{request.reason}</p></details>}</>;
}
