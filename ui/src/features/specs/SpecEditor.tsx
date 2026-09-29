import {StatusBadge} from '@/components/StatusBadge';
import {useEffect,useState,useMemo} from 'react';
import {api,readSpec,errorText,type CanonicalSpec,type ReadResult,type Session} from '@/api/client';
import {Button} from '@/components/ui/button';
import {Textarea} from '@/components/ui/textarea';
import {DataDetails} from './NativeDocument';
const keys=['title','goal','rationale','scope','questions','operations'] as const;
type Fields=Pick<CanonicalSpec,typeof keys[number]>;
const fields=(c:CanonicalSpec):Fields=>Object.fromEntries(keys.map(k=>[k,c[k]])) as unknown as Fields;
const defaultTransport={read:readSpec,write:api};
function patch(base:Fields,draft:Fields){return Object.fromEntries(keys.filter(k=>base[k]!==draft[k]&&JSON.stringify(base[k])!==JSON.stringify(draft[k])).map(k=>[k,draft[k]]))}

export function SpecEditor({read,session,onDirty,onSaved,onCancel,initialField='goal',transport=defaultTransport,onBusy}:{read:ReadResult;session:Session;onDirty:(v:boolean)=>void;onSaved:()=>Promise<void>;onCancel:()=>void;initialField?:string;transport?:typeof defaultTransport;onBusy:(v:boolean)=>void}){
 const [base,setBase]=useState(read);const [draft,setDraft]=useState(()=>fields(read.change));const [errors,setErrors]=useState<Record<string,string>>({});const [message,setMessage]=useState('');const [busy,setBusy]=useState(false);const [newer,setNewer]=useState<ReadResult>();const [uncertain,setUncertain]=useState(false);
 const baseFields=useMemo(()=>fields(base.change),[base]);
 const changes=useMemo(()=>patch(baseFields,draft),[baseFields,draft]);const dirty=Object.keys(changes).length>0;
 useEffect(()=>{onDirty(dirty)},[dirty,onDirty]);
 useEffect(()=>{onBusy(busy);return()=>onBusy(false)},[busy,onBusy]);
 useEffect(()=>{document.getElementById(`edit-${initialField}`)?.focus()},[initialField]);
 const set=(key:typeof keys[number],value:unknown)=>{setDraft(v=>({...v,[key]:value}));setErrors({});setMessage('')};
 const field=(name:string,label:string,value:string,change:(v:string)=>void)=><div key={name}><label htmlFor={`edit-${name}`}>{label}</label><Textarea id={`edit-${name}`} disabled={busy} value={value} onChange={e=>change(e.target.value)} aria-invalid={!!errors[name]} aria-describedby={errors[name]?`error-${name}`:undefined}/>{errors[name]&&<p id={`error-${name}`} className="field-error" role="alert">{errors[name]}</p>}</div>;
 const validate=()=>{const found:Record<string,string>={};for(const k of ['title','goal','scope'] as const)if(!draft[k].trim())found[k]='Заполните поле.';draft.operations.forEach((op,i)=>{if(op.action==='remove'){if(!op.reason.trim())found[`operation-${i}`]='Укажите причину удаления.'}else{for(const k of ['title','statement'] as const)if(!op.requirement[k].trim())found[`requirement-${i}-${k}`]='Заполните поле.';op.requirement.scenarios.forEach((s,j)=>{if(!s.then.trim())found[`criterion-${i}-${j}`]='Заполните поле.'})}});setErrors(found);if(Object.keys(found).length){document.getElementById(`edit-${Object.keys(found)[0]}`)?.focus();return false}return true};
 const inspect=async()=>{setBusy(true);try{const current=await transport.read(read.change.id);setNewer(current);setMessage('Текущее состояние прочитано. Сопоставьте его со своим вводом; повторной записи не было.')}catch(e){setMessage(errorText(e))}finally{setBusy(false)}};
 const save=async()=>{if(!validate()||!dirty)return;setBusy(true);setMessage('');try{
  await transport.write(`/api/specs/${read.change.id}/edit`,{method:'POST',headers:{'Content-Type':'application/json','X-HighGrade-Token':session.token},body:JSON.stringify({expected_local:base.local_sha256,input:changes})});
  const current=await transport.read(read.change.id);if(Object.entries(changes).some(([k,v])=>JSON.stringify(current.change[k as keyof CanonicalSpec])!==JSON.stringify(v))){setNewer(current);setMessage('После ответа обнаружена другая редакция. Сравните состояние; сохранение не объявлено подтверждённым.');return}
  onDirty(false);await onSaved();
 }catch(e){setMessage(errorText(e));setUncertain(true)}finally{setBusy(false)}};
 return <div className="editor" aria-label="Редактирование спецификации">
  {message&&<div className="notice error" role="alert"><p>{message}</p><p>Ваш ввод сохранён в форме. Не повторяйте запись, пока не проверите текущее состояние.</p>{uncertain&&<Button variant="outline" onClick={inspect} disabled={busy}>Перечитать и сравнить</Button>}</div>}
  {newer&&<section className="notice"><h2>Сравнение редакций</h2><div className="diff-grid"><div><b>Сейчас в проекте</b><DataDetails value={fields(newer.change)}/></div><div><b>Мои изменённые поля</b><DataDetails value={changes}/></div></div><div className="inline-actions"><Button variant="outline" disabled={busy||newer.change.archived} onClick={()=>{const mine=changes;setBase(newer);setDraft({...fields(newer.change),...mine});setNewer(undefined);setUncertain(false);setMessage('Новая база выбрана явно. Проверьте поля и нажмите «Сохранить изменения».')}}>Применить мои поля к новой базе</Button><Button variant="outline" onClick={()=>{setBase(newer);setDraft(fields(newer.change));setNewer(undefined);setUncertain(false);setMessage('Загружено фактическое состояние проекта.')}}>Использовать версию проекта</Button></div></section>}
  {(['title','goal','scope'] as const).map((key,i)=>field(key,['Название','Цель','Границы'][i],draft[key],v=>set(key,v)))}
  {draft.operations.map((op,i)=>{const update=(value:typeof op)=>set('operations',draft.operations.map((item,j)=>j===i?value:item));return <fieldset key={i} className="editor-group"><legend><span className="group-heading"><span>Требование {i+1}</span><StatusBadge status={op.action}/></span></legend>{op.action==='remove'?field(`operation-${i}`,'Причина удаления',op.reason,v=>update({...op,reason:v})): <>{field(`requirement-${i}-title`,'Название требования',op.requirement.title,v=>update({...op,requirement:{...op.requirement,title:v}}))}{field(`requirement-${i}-statement`,'Условие требования',op.requirement.statement,v=>update({...op,requirement:{...op.requirement,statement:v}}))}{op.requirement.scenarios.map((s,j)=>field(`criterion-${i}-${j}`,'Условие готовности',s.then,v=>update({...op,requirement:{...op.requirement,scenarios:op.requirement.scenarios.map((item,n)=>n===j?{...item,then:v}:item)}})))}</>}</fieldset>})}
  {draft.questions.map((q,i)=>field(`question-${i}`,`Вопрос ${i+1}`,q,v=>set('questions',draft.questions.map((value,j)=>j===i?v:value))))}
  <div className="editor-actions"><Button onClick={save} disabled={busy||!dirty||uncertain}>Сохранить изменения</Button><Button variant="outline" onClick={onCancel} disabled={busy}>Отмена</Button><span>{busy?'Проверяем состояние…':dirty?'Есть несохранённые правки':'Нет несохранённых правок'}</span></div>
 </div>;
}
