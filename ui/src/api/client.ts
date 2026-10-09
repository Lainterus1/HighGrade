import type {Requirement} from './types';

export const API_VERSION='2';
export type QueueCategory='needs_decision'|'in_work'|'completed'|'historical'|'cancelled'|'unknown';
export interface Handoff {id:string;kind:'question'|'requirements'|'result';reason:string;content_sha256:string;change_sha256:string;inputs_sha256:string;verified_revision?:string;at:number;response:null|{decision:string;author:string;comment:string;at:number;sequence:number}}
export interface Row {id:string;title:string;tags:string[];created_at:number;category:QueueCategory;content_sha256:string;editable:boolean;integrated:boolean;technical_ready:boolean;human:string;requirements_agreement:string;primary_action:Handoff|null;requests:Handoff[];history:Handoff[];diagnostic:string|null;changes_since_agreement:null|{field:string;before:unknown;after:unknown}[]}
export interface Projection {store_sha256:string;changes:Row[];counts:Record<QueueCategory,number>;excluded_count:number;excluded:{id:string;reason:string}[]}
export type Operation={action:'add'|'modify';requirement:Requirement}|{action:'remove';id:string;reason:string};
export interface CanonicalSpec {id:string;title:string;goal:string;rationale:string;scope:string;questions:string[];operations:Operation[];tasks:{id:string;description:string;done:boolean}[];depends_on:{id:string;reason:string}[];related_to:{id:string;reason:string}[];tags:string[];checks:unknown[];runs:unknown[];acceptance:{decision:'accepted'|'needs_changes';decided_by:string;decided_at:number;change_sha256:string;inputs_sha256:string;verified_revision:string;comment:string}[];history:unknown[];evidence:Record<string,{scenario:string;outcome:string;observation:string;command:string;captured_at:string;report:string}>;review:null|{verdict:string;reviewer:string;conclusion:string};archived:boolean;abandoned_reason:string|null}
export interface ReadResult {change:CanonicalSpec;store_sha256:string;local_sha256:string;change_sha256:string;inputs_sha256:string;links:unknown}
export interface Session {api_version:string;token:string;project:{name:string;root:string}}
export class ApiError extends Error {constructor(public code:string,message:string,public status:number){super(message)}}
let pending:Promise<unknown>=Promise.resolve();
export function api<T>(path:string,init?:RequestInit):Promise<T>{
 const run=pending.catch(()=>undefined).then(async()=>{
  init?.signal?.throwIfAborted();
  for(let attempt=0;;attempt++)try{return await request<T>(path,init)}catch(e){
   if((init?.method??'GET')!=='GET'||!(e instanceof ApiError)||e.code!=='StoreBusy'||attempt===7)throw e;
   await retryDelay(Math.min(200*2**attempt,2000),init?.signal);
  }
 });pending=run;return run;
}
function retryDelay(ms:number,signal?:AbortSignal|null){
 return new Promise<void>((resolve,reject)=>{
  if(signal?.aborted){reject(signal.reason);return}
  const abort=()=>{clearTimeout(timer);signal?.removeEventListener('abort',abort);reject(signal?.reason)};
  const timer=setTimeout(()=>{signal?.removeEventListener('abort',abort);resolve()},ms);
  signal?.addEventListener('abort',abort,{once:true});
 });
}
async function request<T>(path:string,init?:RequestInit):Promise<T>{
 const response=await fetch(path,{...init,headers:{'X-HighGrade-Api':API_VERSION,...init?.headers}});
 let data;try{data=await response.json()}catch{throw new ApiError('InvalidResponse','Сервер вернул непонятный ответ.',response.status)}
 if(!response.ok)throw new ApiError(data.error?.code??data.findings?.[0]?.code??'RequestFailed',data.error?.message??data.findings?.map((f:{message:string})=>f.message).join('\n')??'Операция не подтверждена',response.status);
 return (data.result??data) as T;
}
export const readProjection=(signal?:AbortSignal)=>api<Projection>('/api/specs',{signal});
export const readSpec=(id:string,signal?:AbortSignal)=>api<ReadResult>('/api/specs/'+encodeURIComponent(id),{signal});
export function errorText(error:unknown){return error instanceof ApiError?`${error.code}: ${error.message}`:error instanceof Error&&!(error instanceof TypeError)?error.message:'Связь с проектом потеряна. Проверьте, запущен ли HighGrade.'}
