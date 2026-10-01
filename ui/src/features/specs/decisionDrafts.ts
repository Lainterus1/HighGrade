import type {Handoff,ReadResult,Session} from '@/api/client';

export interface DecisionDraft {
 revision:string;
 author:string;
 comment:string;
 verifiedRevision:string;
 uncertain:boolean;
 error:string;
}

// Text and retry guards live only in this page's memory, never in browser storage.
const drafts=new Map<string,DecisionDraft>();
let warningInstalled=false;
const warnBeforeUnload=(event:BeforeUnloadEvent)=>{event.preventDefault();event.returnValue=''};

export const decisionDraftKey=(session:Session,read:ReadResult,request:Handoff)=>
 JSON.stringify([session.project.root,read.change.id,request.id,request.kind]);
export const decisionRevision=(read:ReadResult,request:Handoff)=>
 JSON.stringify([request.kind,request.reason,request.at,request.content_sha256,request.change_sha256,request.inputs_sha256,read.change_sha256,read.inputs_sha256]);

export function readDecisionDraft(key:string,revision:string):DecisionDraft {
 const saved=drafts.get(key);
 if(!saved)return {revision,author:'',comment:'',verifiedRevision:'',uncertain:false,error:''};
 if(saved.revision===revision)return saved;
 return {...saved,revision,verifiedRevision:'',error:saved.verifiedRevision?'Редакция изменилась. Укажите проверенную версию заново.':saved.error};
}

function updateWarning(){
 if(typeof window==='undefined')return;
 if(drafts.size&&!warningInstalled){window.addEventListener('beforeunload',warnBeforeUnload);warningInstalled=true}
 else if(!drafts.size&&warningInstalled){window.removeEventListener('beforeunload',warnBeforeUnload);warningInstalled=false}
}
export function writeDecisionDraft(key:string,draft:DecisionDraft){
 if(draft.author||draft.comment||draft.verifiedRevision||draft.uncertain)drafts.set(key,draft);
 else drafts.delete(key);
 updateWarning();
}
export function clearDecisionDraft(key:string){drafts.delete(key);updateWarning()}
