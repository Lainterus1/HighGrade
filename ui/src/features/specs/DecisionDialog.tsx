import {decisionReason} from './decisionCopy';
import {useEffect,useRef,useState} from 'react';
import {api,readSpec,errorText,type Handoff,type ReadResult,type Session} from '@/api/client';
import {Button} from '@/components/ui/button';
import {Input} from '@/components/ui/input';
import {Textarea} from '@/components/ui/textarea';
import {Dialog,DialogContent,DialogHeader,DialogTitle,DialogDescription,DialogFooter} from '@/components/ui/dialog';
import {decisionDraftKey,decisionRevision,readDecisionDraft,writeDecisionDraft,clearDecisionDraft,type DecisionDraft} from './decisionDrafts';
import {ResultStatusSelect} from './ResultStatusSelect';

type Props={onBusy:(v:boolean)=>void;request:Handoff;read:ReadResult;session:Session;onClose:()=>void;onSaved:()=>Promise<void>};
export function DecisionDialog(props:Props){
 const draftKey=decisionDraftKey(props.session,props.read,props.request);
 const revision=decisionRevision(props.read,props.request);
 // A new target cannot keep another form's state, even without a parent unmount.
 return <DecisionForm key={JSON.stringify([draftKey,revision])} {...props} draftKey={draftKey} revision={revision}/>;
}

function DecisionForm({request,read,session,onClose,onSaved,onBusy,draftKey,revision}:Props&{draftKey:string;revision:string}){
 const [draft,setDraft]=useState(()=>readDecisionDraft(draftKey,revision));
 const draftRef=useRef(draft);
 const [busy,setBusy]=useState(false);
 const busyRef=useRef(false),alive=useRef(true),expected=useRef(read.store_sha256);
 const update=(patch:Partial<DecisionDraft>)=>{
  const next={...draftRef.current,...patch};draftRef.current=next;
  writeDecisionDraft(draftKey,next);setDraft(next);return next;
 };
 useEffect(()=>{alive.current=true;writeDecisionDraft(draftKey,draftRef.current);return()=>{alive.current=false}},[draftKey]);
 useEffect(()=>{onBusy(busy);return()=>onBusy(false)},[busy,onBusy]);
 const title=request.kind==='question'?'Ответить на вопрос':request.kind==='requirements'?'Согласовать требования':'Изменить статус результата';
 const close=()=>{if(!busyRef.current)onClose()};
 const decide=async(decision:string)=>{
  if(busyRef.current||draftRef.current.uncertain)return;
  const {author,comment}=draftRef.current;
  if(request.kind!=='result'&&!author.trim()){update({error:'Укажите ваше имя.'});return}
  if((decision==='answer'||(decision==='needs_changes'&&request.kind!=='result'))&&!comment.trim()){update({error:'Добавьте ответ или пояснение.'});return}
  busyRef.current=true;setBusy(true);
  // Persist before starting the request: dismissal/unmount must not enable a retry.
  update({uncertain:true,error:''});
  try{
   await api(`/api/specs/${read.change.id}/attention`,{method:'POST',headers:{'Content-Type':'application/json','X-HighGrade-Token':session.token},body:JSON.stringify({expected:expected.current,input:{action:'respond',request_id:request.id,content_sha256:request.content_sha256,decision,author:request.kind==='result'?'Локальный интерфейс':author.trim(),comment}})});
   if(!alive.current)return;
   clearDecisionDraft(draftKey);
   await onSaved();if(alive.current)onClose();
  }catch(e){if(alive.current)update({error:errorText(e),uncertain:true})}
  finally{busyRef.current=false;if(alive.current)setBusy(false)}
 };
 const inspect=async()=>{
  if(busyRef.current)return;
  busyRef.current=true;setBusy(true);
  try{
   const current=await readSpec(read.change.id);
   if(!alive.current)return;
   const latest=(current.change as unknown as {attention?:{requests:Handoff[]}}).attention?.requests.find(r=>r.id===request.id);
   if(latest?.response){
    update({uncertain:true,error:`В проекте записано решение: ${latest.response.decision}; источник: ${latest.response.author}. Обновите документ.`});
   }else if(!latest||decisionRevision(current,latest)!==revision){
    update({uncertain:true,error:'Редакция обращения изменилась или обращение недоступно. Закройте окно и обновите документ перед новой попыткой. Черновик решения сохранён.'});
   }else{
    expected.current=current.store_sha256;
    update({uncertain:false,error:'Ответ не найден в текущем состоянии. Проверьте черновик и сохраните решение отдельным нажатием.'});
   }
  }catch(e){if(alive.current)update({error:errorText(e)})}
  finally{busyRef.current=false;if(alive.current)setBusy(false)}
 };
 return <Dialog open onOpenChange={open=>{if(!open)close()}}><DialogContent className={request.kind==='result'?'decision-dialog-compact':undefined}>
  <DialogHeader><DialogTitle>{title}</DialogTitle><DialogDescription>{request.kind==='requirements'?'Согласие относится только к показанному содержанию. Это не разрешение выполнять работу и не запуск агента.':request.kind==='result'?'Выберите статус показанного результата. Комментарий можно оставить пустым.':'Ответ закроет только этот записанный вопрос.'}</DialogDescription></DialogHeader>
  {request.kind!=='result'&&decisionReason(request)&&<p className="source-text">{decisionReason(request)}</p>}
  {request.kind==='result'?<ResultStatusSelect value={draft.status} onChange={status=>update({status})} disabled={busy}/>:<><label htmlFor="decision-author">Ваше имя</label><Input id="decision-author" disabled={busy} value={draft.author} onChange={e=>update({author:e.target.value})}/></>}
  <label htmlFor="decision-comment">{request.kind==='question'?'Ответ':'Комментарий'}</label><Textarea id="decision-comment" disabled={busy} value={draft.comment} onChange={e=>update({comment:e.target.value})}/>
  {draft.error&&<div role="alert" className="notice error"><p>{draft.error}</p>{draft.uncertain&&<Button variant="outline" onClick={inspect} disabled={busy}>Проверить состояние решения</Button>}</div>}
  {/* A remount during an unfinished POST still offers a read-only recovery path. */}
  {draft.uncertain&&!draft.error&&!busy&&<div role="alert" className="notice error"><p>Исход записи не подтверждён. Проверьте состояние решения перед новой попыткой.</p><Button variant="outline" onClick={inspect}>Проверить состояние решения</Button></div>}
  <DialogFooter><Button variant="outline" onClick={close} disabled={busy}>Закрыть</Button>{request.kind==='requirements'&&<Button variant="outline" onClick={()=>decide('needs_changes')} disabled={busy||draft.uncertain}>Вернуть на доработку</Button>}<Button onClick={()=>decide(request.kind==='question'?'answer':request.kind==='result'?draft.status:'accepted')} disabled={busy||draft.uncertain}>{request.kind==='question'?'Сохранить ответ':request.kind==='result'?'Сохранить статус':'Подтвердить'}</Button></DialogFooter>
 </DialogContent></Dialog>;
}
