import {StatusBadge} from '@/components/StatusBadge';
import type {ReadResult,Row} from '@/api/client';
import {Button} from '@/components/ui/button';
import {Accordion,AccordionItem,AccordionTrigger,AccordionContent} from '@/components/ui/accordion';

/** React text nodes deliberately preserve source text without interpreting HTML or remote media. */
export function SourceText({value}:{value:string|undefined}){return <p className="source-text">{value?.trim()?value:<span className="subtle">Не указано</span>}</p>}
export function displayLabel(value:string){return value.replace(/HG-(?:\d+|DEMO-[A-Z0-9]+)-(R|S|T)(\d+)/g,(_,kind,n)=>`${({R:'Требование',S:'Сценарий',T:'Задача'} as Record<string,string>)[kind]} ${n}`)}
export function DataDetails({value}:{value:unknown}){return <pre className="source-json">{JSON.stringify(value,null,2)}</pre>}
export function NativeDocument({read,row,expanded,onExpanded,onEdit}:{onEdit?:(field:string)=>void;read:ReadResult;row:Row;expanded:string[];onExpanded:(v:string[])=>void}){
 const c=read.change;
 const criteria=[...new Set(c.operations.flatMap(op=>op.action==='remove'?[]:op.requirement.scenarios.map(s=>s.then)))];
 return <>
  <div className="sections spec-sections">
   <section className="section spec-goal"><div className="section-heading"><h2>Цель</h2>{onEdit&&<Button variant="ghost" size="sm" onClick={()=>onEdit('goal')} aria-label="Изменить: Цель">Изменить</Button>}</div><SourceText value={c.goal}/></section>
   <section className="section spec-changes"><div className="section-heading"><h2>Что изменится</h2>{onEdit&&<Button variant="ghost" size="sm" onClick={()=>onEdit('requirement-0-statement')} aria-label="Изменить: Требования">Изменить</Button>}</div><Accordion type="multiple" value={expanded} onValueChange={onExpanded}>{c.operations.map(op=>op.action==='remove'?<div className="requirement" key={op.id}><div><div className="group-heading"><h3>Требование</h3><StatusBadge status="remove"/></div><SourceText value={op.reason}/></div></div>:<AccordionItem value={`requirement-${op.requirement.id}`} key={op.requirement.id}><AccordionTrigger><span className="group-heading"><span>{op.requirement.title}</span><StatusBadge status={op.action}/></span></AccordionTrigger><AccordionContent><SourceText value={op.requirement.statement}/></AccordionContent></AccordionItem>)}</Accordion></section>
   <section className="section spec-scope"><div className="section-heading"><h2>Границы и ограничения</h2>{onEdit&&<Button variant="ghost" size="sm" onClick={()=>onEdit('scope')} aria-label="Изменить: Границы">Изменить</Button>}</div><SourceText value={c.scope}/></section>
   <section className="section spec-criteria"><h2>Готово, когда</h2><ul className="section-list criteria-list">{criteria.map((value,i)=><li key={i}><SourceText value={value}/></li>)}</ul>{!criteria.length&&<p className="subtle">Критерии не указаны.</p>}</section>
   {c.abandoned_reason&&<section className="notice"><h2>Отменена</h2><SourceText value={c.abandoned_reason}/></section>}
  </div>
  <Accordion type="multiple" value={expanded} onValueChange={onExpanded} className="details">
   {!!c.questions.length&&<section className="section"><h2>Вопросы</h2>{c.questions.map((q,i)=><SourceText key={i} value={q}/>)}</section>}
   {row.changes_since_agreement&&<AccordionItem value="diff" data-diff-section><AccordionTrigger>Изменения после согласования</AccordionTrigger><AccordionContent>{row.changes_since_agreement.length?row.changes_since_agreement.map(v=><section key={v.field}><h3>{displayLabel(v.field)}</h3><div className="diff-grid"><div><b>Было</b><DataDetails value={v.before}/></div><div><b>Стало</b><DataDetails value={v.after}/></div></div></section>):<p>Содержательных изменений нет.</p>}</AccordionContent></AccordionItem>}
  </Accordion>
 </>;
}
