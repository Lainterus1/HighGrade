import {NativeDocument} from './NativeDocument';
import {ResultStatusSelect} from './ResultStatusSelect';
import type {ReadResult,Row} from '@/api/client';
import {StatusBadge} from '@/components/StatusBadge';
import {useEffect,useRef,useState} from 'react';
import type {Category, ScreenState, SpecDocument} from '@/api/types';
import {demoRows,demoSpec,demoDocuments} from './fixtures';
import {Button} from '@/components/ui/button';
import {Input} from '@/components/ui/input';
import {Textarea} from '@/components/ui/textarea';
import {Accordion,AccordionItem,AccordionTrigger,AccordionContent} from '@/components/ui/accordion';
import {Dialog,DialogContent,DialogHeader,DialogTitle,DialogDescription,DialogFooter} from '@/components/ui/dialog';
import {GradeIcon,Search,Pencil,Folder,Menu,ListFilter,CircleAlert,WifiOff,FileQuestion} from '@/components/icons';
import wordmark from '@/assets/brand/wordmark.svg';

export interface WorkspaceProps {state?:ScreenState; document?:SpecDocument;project?:{name:string;root:string}}
const nav:{id:Category;label:string}[]=[{id:'all',label:'Все спецификации'},{id:'decision',label:'Нужно решение'},{id:'work',label:'В работе'},{id:'completed',label:'Завершены'}];
const names={approval:'Согласовать требования',question:'Ответить на вопрос',acceptance:'Изменить статус'};
export function SpecWorkspace({state:initialState='approval',document:source=demoSpec,project={name:'MyCodex',root:'Projects/MyCodex'}}:WorkspaceProps) {
 const [state,setState]=useState(initialState);
 if(state==='long')source={...source,requirements:Array.from({length:9},(_,i)=>({...source.requirements[i%3],id:`HG-DEMO-LONG-R${i+1}`,scenarios:source.requirements[i%3].scenarios.map((s,j)=>({...s,id:`HG-DEMO-LONG-S${i+1}-${j+1}`}))}))};
 const [doc,setDoc]=useState(source);const [draft,setDraft]=useState(source);const [category,setCategory]=useState<Category>('decision');const [search,setSearch]=useState(state==='search-empty'?'ничего не найдено':'');const [editing,setEditing]=useState(['editing','conflict','validation'].includes(state));const [dirty,setDirty]=useState(false);const [menu,setMenu]=useState(false);const [width,setWidth]=useState(288);const [dialog,setDialog]=useState<'decision'|'discard'|null>(null);const [answer,setAnswer]=useState('');const [message,setMessage]=useState('');const [resultStatus,setResultStatus]=useState<'accepted'|'needs_changes'>('accepted');const [resultComment,setResultComment]=useState('');const [error,setError]=useState(state==='validation'?'Поле «Цель» не может быть пустым.':'');const [conflict,setConflict]=useState(state==='conflict');
 const searchRef=useRef<HTMLInputElement>(null);const editRef=useRef<HTMLButtonElement>(null);
 useEffect(()=>{if(editing)document.getElementById('field-goal')?.focus()},[editing]);
 useEffect(()=>{const listener=(e:KeyboardEvent)=>{if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='k'){e.preventDefault();setMenu(true);searchRef.current?.focus()}};window.addEventListener('keydown',listener);return()=>window.removeEventListener('keydown',listener)},[]);
 const needsDecision=['approval','question','acceptance'].includes(state)&&!message&&!editing;
 const decisionKind=state==='question'?'question':state==='acceptance'?'acceptance':'approval';
 const availableRows=state==='empty'?[]:demoRows;
 const rows=availableRows.filter(r=>(category==='all'||r.category===category)&&`${r.id} ${r.title}`.toLowerCase().includes(search.toLowerCase()));
 const [selected,setSelected]=useState(doc.id);const [expanded,setExpanded]=useState<string[]>([]);
 const nativeRead:ReadResult={store_sha256:'demo',local_sha256:'demo',change_sha256:'demo',inputs_sha256:'demo',links:{},change:{...doc,questions:[],operations:doc.requirements.map(requirement=>({action:'add',requirement})),depends_on:[],related_to:[],tags:[],checks:[],runs:[],acceptance:[],history:[],evidence:{},review:null,archived:state==='integrated',abandoned_reason:null}};
 const nativeRow={changes_since_agreement:null} as Row;
 const discard=()=>{setEditing(false);setDirty(false);setDraft(doc);setDialog(null);setError('');requestAnimationFrame(()=>editRef.current?.focus())};
 const save=()=>{if(!draft.goal.trim()){setError('Поле «Цель» не может быть пустым.');return}if(conflict)return;setDoc(draft);setEditing(false);setDirty(false);setMessage('Изменения сохранены в демонстрации. Файлы проекта не изменены.');requestAnimationFrame(()=>editRef.current?.focus())};
 const update=(key:'goal'|'rationale'|'scope',value:string)=>{setDraft({...draft,[key]:value});setDirty(true);setError('')};
 const emptyInfo:Partial<Record<ScreenState,{title:string;copy:string}>>={empty:{title:'Здесь будут спецификации',copy:'Каталог проекта пока пуст. Когда спецификация появится в проекте, она будет доступна здесь.'},unsupported:{title:'Для этой структуры доступен CLI',copy:'Одна спецификация использует прежний формат. Она сохранена без изменений и не включена в рабочий список.'},corrupt:{title:'Не удалось прочитать каталог',copy:'Файл спецификации повреждён. Это ошибка чтения, а не пустой проект. Данные не изменены.'},offline:{title:'Связь с проектом потеряна',copy:'Проверьте, запущен ли HighGrade. Несохранённый текст остаётся в этом окне.'}};
 const empty=emptyInfo[state];const long=state==='long';
 return <div className={`workspace ${menu?'menu-open':''}`} style={{'--sidebar-width':`${width}px`} as React.CSSProperties}>
  <aside className="sidebar" aria-label="Навигация проекта">
   <div className="brand"><img src={wordmark} alt="HighGrade"/></div>
   <div><div className="project-name"><Folder size={18} aria-hidden/>{project.name}</div><div className="search"><Search size={17} aria-hidden/><Input ref={searchRef} aria-label="Поиск по спецификациям" placeholder="Поиск по спекам" value={search} onChange={e=>setSearch(e.target.value)}/><kbd>Ctrl K</kbd></div></div>
   <nav className="nav" aria-label="Категории спецификаций">{nav.map(n=><button key={n.id} type="button" aria-pressed={category===n.id} onClick={()=>setCategory(n.id)}><GradeIcon name={n.id}/>{n.label}<span className="count">{n.id==='all'?availableRows.length:availableRows.filter(r=>r.category===n.id).length}</span></button>)}</nav>
   <div className="spec-list"><div className="list-heading"><span>{category==='decision'?'Ждут вашего решения':'Спецификации'}</span><ListFilter size={15} aria-hidden/></div>{rows.map(r=><button key={r.id} className="spec-row" aria-current={selected===r.id} onClick={()=>{if(dirty){setDialog('discard');return}setSelected(r.id);setDoc(demoDocuments[r.id]);setDraft(demoDocuments[r.id]);setState(({ 'HG-DEMO-01':'approval','HG-DEMO-02':'acceptance','HG-DEMO-03':'question','HG-DEMO-04':'reading','HG-DEMO-05':'integrated'} as Record<string,ScreenState>)[r.id]);setEditing(false);setMenu(false);setMessage('')}}><span className="spec-id">{r.id}</span><span className="spec-title">{r.title}</span><StatusBadge status={r.category==='decision'?'needs_decision':r.category==='work'?'in_work':'completed'}/></button>)}{!rows.length&&<p className="subtle" role="status">Ничего не найдено. Попробуйте другое название или ID.</p>}</div>
   <footer className="sidebar-footer"><span className="demo-pill">Демонстрационные данные</span><p>{project.root}</p><p>Один проект · локальный интерфейс</p></footer>
  </aside>
  <div className="resize-handle" role="separator" tabIndex={0} aria-label="Ширина списка спецификаций" aria-orientation="vertical" aria-valuemin={240} aria-valuemax={380} aria-valuenow={width} onKeyDown={e=>{if(e.key==='ArrowLeft'||e.key==='ArrowRight'){e.preventDefault();setWidth(w=>Math.max(240,Math.min(380,w+(e.key==='ArrowRight'?10:-10))))}}} onPointerDown={e=>e.currentTarget.setPointerCapture(e.pointerId)} onPointerMove={e=>{if(e.currentTarget.hasPointerCapture(e.pointerId))setWidth(Math.max(240,Math.min(380,e.clientX)))}}/>
  <main className="main"><header className="toolbar"><div className="crumb"><Button className="mobile-toggle" variant="ghost" size="icon" aria-label="Открыть список спецификаций" onClick={()=>setMenu(v=>!v)}><Menu size={19}/></Button><span>Спецификации</span><span aria-hidden>/</span><span>{doc.id}</span></div>{!editing&&!empty&&state!=='loading'&&<Button ref={editRef} variant="outline" onClick={()=>{setDraft(doc);setEditing(true);setMessage('')}} disabled={state==='integrated'}><Pencil size={15} aria-hidden/>{state==='integrated'?'Только чтение':'Редактировать'}</Button>}</header><div className="mobile-context"><span className="demo-pill">Демонстрационные данные</span><p>{project.name} · {project.root}</p></div>
   {empty?<div className="empty-state">{state==='offline'?<WifiOff size={32} aria-hidden/>:state==='corrupt'?<CircleAlert size={32} aria-hidden/>:<FileQuestion size={32} aria-hidden/>}<h1>{empty.title}</h1><p>{empty.copy}</p><span className="demo-pill">Демонстрационное состояние</span></div>:state==='loading'?<div className="document" role="status" aria-label="Загрузка спецификации"><h1>Открываем спецификацию…</h1>{Array.from({length:8},(_,i)=><div className="skeleton" key={i}/>)}</div>:<article className="document spec-document">
    <div className="eyebrow"><span>{doc.id}</span><span>·</span>{!needsDecision&&<StatusBadge status={state==='integrated'?'completed':'in_work'}/>}
</div>
    <h1>{long?'Безопасное редактирование спецификаций и согласование требований при параллельной работе в нескольких окнах':doc.title}</h1>
    {message&&<div className="notice" role="status">{message}</div>}
    {state==='stale'&&<div className="notice" role="status"><strong>В проекте появилась новая редакция</strong><p>Сейчас открыта прочитанная вами версия. Обновление не заменит текст незаметно.</p><Button variant="outline" onClick={()=>setMessage('В демонстрации показано место для загрузки новой редакции.')}>Посмотреть изменения</Button></div>}
    {needsDecision&&<section className={`decision ${decisionKind==='acceptance'?'decision-compact':''}`} aria-label="Ожидаемое решение"><div><h2>{decisionKind==='question'?'Нужно уточнить поведение':decisionKind==='acceptance'?'Результат готов':'Требования готовы к согласованию'}</h2>{decisionKind!=='acceptance'&&<p>{decisionKind==='question'?'Как поступать с несохранёнными правками при переходе к другой спеке?':'Проверьте содержание ниже. Согласование относится к этой редакции требований.'}</p>}</div><Button onClick={()=>setDialog('decision')}>{names[decisionKind]}</Button></section>}
    {editing?<div className="editor">
     {conflict&&<div className="notice error" role="alert"><strong>Документ изменился в проекте</strong><p>Ваш текст сохранён в форме. Сопоставьте версии перед повторным сохранением.</p><div className="diff-grid"><p><b>В проекте</b><br/>Просматривать актуальные спецификации проекта.</p><p><b>Ваш вариант</b><br/>{draft.goal}</p></div><Button variant="outline" onClick={()=>{setConflict(false);setMessage('Версии сопоставлены в демонстрации. Теперь можно сохранить выбранный текст.')}}>Я сопоставил версии</Button></div>}
     {(['goal','scope'] as const).map((key,i)=><div key={key}><label htmlFor={`field-${key}`}>{['Цель','Границы'][i]}</label><Textarea id={`field-${key}`} value={draft[key]} onChange={e=>update(key,e.target.value)} aria-invalid={key==='goal'&&!!error} aria-describedby={key==='goal'&&error?'goal-error':undefined}/>{key==='goal'&&error&&<p id="goal-error" role="alert" className="subtle">{error}</p>}</div>)}
     <div><label htmlFor="field-requirement">Требование · {draft.requirements[0].title}</label><Textarea id="field-requirement" value={draft.requirements[0].statement} onChange={e=>{setDraft({...draft,requirements:draft.requirements.map((r,i)=>i? r:{...r,statement:e.target.value})});setDirty(true)}}/></div>
     <div className="editor-actions"><Button onClick={save} disabled={conflict}>Сохранить изменения</Button><Button variant="outline" onClick={()=>dirty?setDialog('discard'):discard()}>Отмена</Button><span>{dirty?'Есть несохранённые правки':'Демонстрация · запись в проект отключена'}</span></div>
    </div>:<>
     {state==='diff'&&<section className="notice"><h2>Изменения после согласования</h2><p>Точное сравнение поля «Границы» со снимком согласованной редакции.</p><div className="diff-grid"><p><b>Было</b><br/>Просмотр существующих спецификаций.</p><p><b>Стало</b><br/>{doc.scope}</p></div></section>}
     <NativeDocument read={nativeRead} row={nativeRow} expanded={expanded} onExpanded={setExpanded}/>

    </>}
   </article>}
  </main>
  <Dialog open={dialog!==null} onOpenChange={open=>{if(!open)setDialog(null)}}><DialogContent>
   <DialogHeader><DialogTitle>{dialog==='discard'?'Отменить несохранённые правки?':decisionKind==='acceptance'?'Изменить статус результата':names[decisionKind]}</DialogTitle><DialogDescription>{dialog==='discard'?'Восстановится текст до редактирования.':decisionKind==='approval'?'Вы соглашаетесь с содержанием требований. Это не разрешение выполнять работу и не запуск агента.':decisionKind==='acceptance'?'Выберите статус показанного результата. Комментарий можно оставить пустым.':'Ответ сохранится только для выбранного вопроса.'}</DialogDescription></DialogHeader>
   {dialog==='decision'&&decisionKind==='acceptance'&&<><ResultStatusSelect value={resultStatus} onChange={setResultStatus}/><label htmlFor="result-comment">Комментарий</label><Textarea id="result-comment" value={resultComment} onChange={event=>setResultComment(event.target.value)}/></>}
   {dialog==='decision'&&decisionKind==='question'&&<div className="dialog-field"><label htmlFor="answer">Ваш ответ</label><Textarea id="answer" value={answer} onChange={e=>setAnswer(e.target.value)}/></div>}
   <p className="dialog-copy">Демонстрационные данные. Файлы проекта не изменяются.</p>
   <DialogFooter><Button variant="outline" onClick={()=>setDialog(null)}>{dialog==='discard'?'Продолжить редактирование':'Отмена'}</Button><Button disabled={dialog==='decision'&&decisionKind==='question'&&!answer.trim()} onClick={()=>{if(dialog==='discard'){discard();return}setMessage('Решение записано в демонстрации. Реальная спецификация не изменена.');setDialog(null)}}>{dialog==='discard'?'Отменить правки':decisionKind==='question'?'Сохранить ответ':decisionKind==='acceptance'?'Сохранить статус':'Подтвердить'}</Button></DialogFooter>
  </DialogContent></Dialog>
 </div>;
}
