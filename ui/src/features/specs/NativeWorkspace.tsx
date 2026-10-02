import {decisionReason} from './decisionCopy';
import {StatusBadge} from '@/components/StatusBadge';
import {useCallback,useEffect,useRef,useState,useMemo} from 'react';
import {api,readProjection,readSpec,errorText,API_VERSION,type Projection,type ReadResult,type Row,type Session} from '@/api/client';
import {Button} from '@/components/ui/button';
import {Input} from '@/components/ui/input';
import {GradeIcon,Search,Folder,Menu} from '@/components/icons';
import {SpecEditor} from './SpecEditor';
import {DecisionDialog} from './DecisionDialog';
import {Dialog,DialogContent,DialogHeader,DialogTitle,DialogDescription,DialogFooter} from '@/components/ui/dialog';
import type {Handoff} from '@/api/client';
import {NativeDocument} from './NativeDocument';
import wordmark from '@/assets/brand/wordmark.svg';

type Category='all'|'needs_decision'|'in_work'|'completed';
interface Snapshot {read:ReadResult;row:Row}
const DOCUMENT_CACHE_LIMIT=10;
const sameDocument=(a:Snapshot,b:Snapshot)=>JSON.stringify([a.read.change,a.read.links,a.read.inputs_sha256,a.row])===JSON.stringify([b.read.change,b.read.links,b.read.inputs_sha256,b.row]);
const categories=[['all','Все спецификации','all'],['needs_decision','Нужно решение','decision'],['in_work','В работе','work'],['completed','Завершены','completed']] as const;
const visibleRows=(list:Projection|undefined,category:Category,cancelled:boolean,tag:string,search:string)=>{
 const query=search.toLocaleLowerCase();
 return (list?.changes??[]).filter(r=>(category==='all'?(cancelled||r.category!=='cancelled'):r.category===category)&&(!tag||r.tags.includes(tag))&&`${r.id} ${r.title}`.toLocaleLowerCase().includes(query));
};
const actionNames={question:'Ответить на вопрос',requirements:'Согласовать требования',result:'Принять результат'};
export const nativeClient={session:()=>api<Session>('/api/session'),list:readProjection,read:readSpec};

export function NativeWorkspace({client=nativeClient}:{client?:typeof nativeClient}){
 const [writeBusy,setWriteBusy]=useState(false);
 const [editing,setEditing]=useState(false);const [dirty,setDirty]=useState(false);const [initialField,setInitialField]=useState('goal');const [pending,setPending]=useState<(()=>void)|null>(null);const [decision,setDecision]=useState<Handoff|null>(null);const [message,setMessage]=useState('');const [diffReviewed,setDiffReviewed]=useState(false);
 const guard=(action:()=>void)=>{if(writeBusy)return;if(dirty)setPending(()=>action);else action()};
 useEffect(()=>{const leave=(e:BeforeUnloadEvent)=>{if(dirty||writeBusy){e.preventDefault();e.returnValue=''}};window.addEventListener('beforeunload',leave);return()=>window.removeEventListener('beforeunload',leave)},[dirty,writeBusy]);
 const [session,setSession]=useState<Session>();const [projection,setProjection]=useState<Projection>();
 const [snapshot,setSnapshot]=useState<Snapshot>();const [category,setCategory]=useState<Category>('in_work');
 const [search,setSearch]=useState('');const [tag,setTag]=useState('');const [sort,setSort]=useState('attention');const [cancelled,setCancelled]=useState(false);
 const [error,setError]=useState('');const [loading,setLoading]=useState(true);const [stale,setStale]=useState(false);const [menu,setMenu]=useState(false);const [width,setWidth]=useState(288);
 const [expanded,setExpanded]=useState<string[]>([]);const cache=useRef(new Map<string,Snapshot>());const places=useRef(new Map<string,{scroll:number;expanded:string[]}>());
 const snapshotRef=useRef(snapshot);snapshotRef.current=snapshot;const expandedRef=useRef(expanded);expandedRef.current=expanded;const searchRef=useRef<HTMLInputElement>(null);const request=useRef(0);const alive=useRef(true);const polling=useRef(false);
 const remember=()=>{if(snapshotRef.current&&selectedRef.current===snapshotRef.current.read.change.id)places.current.set(snapshotRef.current.read.change.id,{scroll:window.scrollY,expanded:expandedRef.current})};
 const show=useCallback((value:Snapshot)=>{const id=value.read.change.id;cache.current.delete(id);cache.current.set(id,value);if(cache.current.size>DOCUMENT_CACHE_LIMIT)cache.current.delete(cache.current.keys().next().value!);setSnapshot(value);snapshotRef.current=value;setExpanded(places.current.get(value.read.change.id)?.expanded??[]);setStale(false);setMenu(false);setDiffReviewed(false);requestAnimationFrame(()=>window.scrollTo(0,places.current.get(value.read.change.id)?.scroll??0))},[]);
 const loadingRef=useRef(loading);const projectionRef=useRef(projection);projectionRef.current=projection;
 const selectionRead=useRef<AbortController|null>(null);
 const [selectedId,setSelectedId]=useState('');const selectedRef=useRef('');
 const load=useCallback(async(id:string,fresh=false,knownList?:Projection)=>{
  if(!fresh&&loadingRef.current&&selectedRef.current===id)return;
  selectionRead.current?.abort();const controller=new AbortController();selectionRead.current=controller;
  const serial=++request.current;setError('');setSelectedId(id);selectedRef.current=id;
  loadingRef.current=true;setLoading(true);
  if(!fresh&&cache.current.has(id))show(cache.current.get(id)!);
  try{
   // Both representations must describe the same catalog snapshot.
   let list=knownList??(!fresh?projectionRef.current:undefined),read:ReadResult|undefined;
   for(let i=0;i<3;i++){
    if(!alive.current||serial!==request.current)return;
    read=await client.read(id,controller.signal);
    if(!alive.current||serial!==request.current)return;
    if(!list||list.store_sha256!==read.store_sha256)list=await client.list(controller.signal);
    if(list.store_sha256===read.store_sha256)break;read=undefined;
   }
   if(!read||!list)throw Error('Каталог меняется; повторите чтение.');
   const row=list.changes.find(r=>r.id===id);if(!row)throw Error('Эта спецификация больше не входит в поддерживаемый список.');
   if(!alive.current||serial!==request.current)return;
   setProjection(list);const value={read,row};
   if(!fresh&&snapshotRef.current?.read.change.id===id){setStale(!sameDocument(snapshotRef.current,value))}else{show(value)}
  }catch(e){if(alive.current&&serial===request.current)setError(errorText(e))}finally{if(alive.current&&serial===request.current){loadingRef.current=false;setLoading(false)}}
 },[client,show]);
 useEffect(()=>{let cancelled=false;alive.current=true;(async()=>{try{const current=await client.session();if(current.api_version!==API_VERSION)throw Error('Версии страницы и сервера различаются.');const list=await client.list();if(cancelled)return;setSession(current);setProjection(list);const initial:Category=list.counts.needs_decision?'needs_decision':list.counts.in_work?'in_work':'all';setCategory(initial);const first=visibleRows(list,initial,false,'','')[0];if(first)await load(first.id,false,list);else setLoading(false)}catch(e){if(!cancelled){setError(errorText(e));setLoading(false)}}})();return()=>{cancelled=true;alive.current=false;selectionRead.current?.abort();request.current++}},[client,load]);
 useEffect(()=>{const poll=async()=>{if(polling.current||document.hidden||loadingRef.current)return;polling.current=true;try{const list=await client.list();if(!alive.current)return;setProjection(list);setError('');const old=snapshotRef.current;if(old){const row=list.changes.find(r=>r.id===old.read.change.id);if(!row)setStale(true);else if(list.store_sha256!==old.read.store_sha256||JSON.stringify(row)!==JSON.stringify(old.row)){const read=await client.read(old.read.change.id);if(alive.current&&snapshotRef.current===old)setStale(!sameDocument(old,{read,row}))}}}catch(e){if(alive.current)setError(errorText(e))}finally{polling.current=false}};const timer=setInterval(poll,10000);window.addEventListener('focus',poll);return()=>{clearInterval(timer);window.removeEventListener('focus',poll)}},[client]);
 useEffect(()=>{const key=(e:KeyboardEvent)=>{if((e.ctrlKey||e.metaKey)&&e.key.toLowerCase()==='k'){e.preventDefault();setMenu(true);requestAnimationFrame(()=>searchRef.current?.focus())}};window.addEventListener('keydown',key);return()=>window.removeEventListener('keydown',key)},[]);
 const rows=useMemo(()=>{
  const result=visibleRows(projection,category,cancelled,tag,search);
  if(sort==='date')result.sort((a,b)=>b.created_at-a.created_at||a.id.localeCompare(b.id));
  return result;
 },[projection,category,cancelled,tag,search,sort]);
 const tags=useMemo(()=>[...new Set(projection?.changes.flatMap(r=>r.tags)??[])].sort(),[projection]);const c=snapshot?.read.change;const primary=snapshot?.row.primary_action;
 const chooseCategory=(next:Category)=>guard(()=>{
  setCategory(next);
  const available=visibleRows(projection,next,cancelled,tag,search);
  if(available.some(r=>r.id===selectedId))return;
  remember();setEditing(false);setDirty(false);setMessage('');
  if(available[0])void load(available[0].id);
  else{selectionRead.current?.abort();request.current++;setSelectedId('');selectedRef.current='';setSnapshot(undefined);snapshotRef.current=undefined;setLoading(false);loadingRef.current=false}
 });
 const integratedNote=!snapshot?.row.integrated||snapshot.row.category==='completed'?'':snapshot.row.category==='unknown'?'Требования интегрированы; входы текущей проверки недоступны, готовность не подтверждена.':snapshot.row.category==='historical'?(snapshot.row.technical_ready?'Требования интегрированы; проверка актуальна, но результат пока не принят для неё.':'Требования интегрированы; текущая проверка устарела.'):'Требования интегрированы; актуальное обращение требует решения.';
 const lastAcceptance=c?.acceptance.at(-1);
 const acceptanceNote=!snapshot?.row.integrated||snapshot.row.category==='completed'?'':!lastAcceptance?'Прежней приёмки результата нет.':`${lastAcceptance.decision==='accepted'?'Прежнее решение: результат принят.':'Прежнее решение: требуются изменения.'} ${snapshot.row.human==='stale'?'Сейчас это решение неактуально.':snapshot.row.human==='needs_changes'?'Результат не принят.':''}`;
 const retry=()=>guard(()=>{remember();setEditing(false);setDirty(false);if(selectedRef.current)void load(selectedRef.current,true);else window.location.reload()});
 const beginEdit=(field='goal')=>{remember();setInitialField(field);setEditing(true);setMessage('')};
 const saved=async()=>{setDirty(false);setEditing(false);if(c)await load(c.id,true);setMessage('Состояние сохранено.')};
 const compareFirst=primary?.kind==='requirements'&&snapshot?.row.requirements_agreement==='stale'&&!!snapshot.row.changes_since_agreement?.length&&!diffReviewed;
 return <div className={`workspace ${menu?'menu-open':''}`} style={{'--sidebar-width':`${width}px`} as React.CSSProperties}>
  <aside className="sidebar" aria-label="Навигация проекта"><div className="brand"><img src={wordmark} alt="HighGrade"/></div><div><div className="project-name"><Folder size={18} aria-hidden/>{session?.project.name??'Проект'}</div><div className="search"><Search size={17} aria-hidden/><Input ref={searchRef} aria-label="Поиск по спецификациям" value={search} onChange={e=>setSearch(e.target.value)} placeholder="Поиск по спекам"/><kbd>Ctrl K</kbd></div></div>
   <nav className="nav" aria-label="Категории спецификаций">{categories.map(([key,label,icon])=><button key={key} disabled={!projection} aria-pressed={category===key} onClick={()=>chooseCategory(key)}><GradeIcon name={icon}/>{label}<span className="count">{key==='all'?projection?.changes.length??0:projection?.counts[key]??0}</span></button>)}</nav>
   <div className="list-filters"><label>Тема<select aria-label="Тема" value={tag} onChange={e=>setTag(e.target.value)}><option value="">Все темы</option>{tags.map(t=><option key={t}>{t}</option>)}</select></label><label>Порядок<select aria-label="Порядок" value={sort} onChange={e=>setSort(e.target.value)}><option value="attention">По вниманию</option><option value="date">Сначала новые</option></select></label>{category==='all'&&<label><input type="checkbox" checked={cancelled} onChange={e=>setCancelled(e.target.checked)}/> Показать отменённые</label>}{(search||tag)&&<Button variant="ghost" onClick={()=>{setSearch('');setTag('')}}>Сбросить фильтры</Button>}</div>
   <div className="spec-list">{rows.map(row=><button className="spec-row" disabled={writeBusy} key={row.id} aria-current={selectedId===row.id} onClick={()=>guard(()=>{remember();setEditing(false);setDirty(false);setMessage('');void load(row.id)})}><span className="spec-id">{row.id}</span><span className="spec-title">{row.title}</span><StatusBadge status={row.category}/></button>)}{!loading&&!rows.length&&<p role="status" className="subtle">{search||tag?'Ничего не найдено. Измените фильтры.':category==='needs_decision'?'Сейчас решений не требуется. Работа может продолжаться.':'В этом разделе пока нет спецификаций.'}</p>}</div>
   {!!projection?.excluded_count&&<details className="subtle"><summary>Прежний формат: {projection.excluded_count}</summary><p>Эти записи сохранены и доступны через CLI. Перенос не выполнялся.</p>{projection.excluded.map(e=><p key={e.id}>{e.id}: {e.reason}</p>)}</details>}
   <footer className="sidebar-footer"><p>{session?.project.root}</p></footer>
  </aside>
  <div className="resize-handle" role="separator" tabIndex={0} aria-label="Ширина списка спецификаций" aria-orientation="vertical" aria-valuemin={240} aria-valuemax={380} aria-valuenow={width} onKeyDown={e=>{if(['ArrowLeft','ArrowRight'].includes(e.key)){e.preventDefault();setWidth(w=>Math.max(240,Math.min(380,w+(e.key==='ArrowRight'?10:-10))))}}} onPointerDown={e=>e.currentTarget.setPointerCapture(e.pointerId)} onPointerMove={e=>{if(e.currentTarget.hasPointerCapture(e.pointerId))setWidth(Math.max(240,Math.min(380,e.clientX)))}}/>
  <main className="main"><header className="toolbar"><div className="crumb"><Button className="mobile-toggle" variant="ghost" size="icon" aria-label="Открыть список спецификаций" onClick={()=>setMenu(v=>!v)}><Menu size={19}/></Button><span>Спецификации</span><span aria-hidden>/</span><span>{c?.id??'Чтение проекта'}</span></div><div className="inline-actions"><Button variant="outline" onClick={retry} disabled={loading||writeBusy}>Обновить</Button>{snapshot&&<Button variant="outline" disabled={!snapshot.row.editable||editing||loading||writeBusy} onClick={()=>beginEdit()}>Редактировать</Button>}</div></header><div className="mobile-context"><p>{session?.project.name} · {session?.project.root}</p></div>
   {error&&<div className="notice error" role="alert"><strong>Данные сейчас недоступны</strong><p>{error}</p><p>Если документ уже открыт, ниже сохранена прочитанная версия.</p><Button variant="outline" onClick={retry}>Повторить чтение</Button></div>}
   {loading&&<p className="loading-notice" role="status">Открываем {selectedId}…</p>}
   {snapshot&&c&&(!loading||c.id===selectedId)?<article className="document spec-document"><div className="eyebrow"><span>{c.id}</span>{snapshot.row.category!=='needs_decision'&&<StatusBadge status={snapshot.row.category}/>}</div><h1>{c.title}</h1>
    {integratedNote&&<p className="subtle status-context">{integratedNote}</p>}
    {acceptanceNote&&<p className="subtle acceptance-context">{acceptanceNote}</p>}
    {message&&<div className="notice" role="status">{message}<Button variant="outline" onClick={()=>{const next=projection?.changes.find(r=>r.primary_action&&r.id!==c.id);if(next){remember();void load(next.id)}else setMessage('Сейчас других обращений нет. Работа может продолжаться.')}}>Следующее обращение</Button></div>}
    {stale&&<div className="notice" role="status"><strong>В проекте появилась новая редакция</strong><p>Показана ранее прочитанная версия. Обновите её явно.</p><Button variant="outline" onClick={retry}>Загрузить новую редакцию</Button></div>}
    {primary&&<section className={`decision ${primary.kind==='result'?'decision-compact':''}`} aria-label="Ожидаемое решение"><div><h2>{primary.kind==='result'?'Результат готов':'Нужно ваше решение'}</h2>{decisionReason(primary)&&<p className="source-text">{decisionReason(primary)}</p>}{editing&&<p>Сохраните или отмените правки перед решением.</p>}{snapshot.row.changes_since_agreement===null&&snapshot.row.requirements_agreement==='stale'&&<p>Снимок прежнего согласования недоступен. Сравнение не построено.</p>}</div><Button disabled={editing||stale||loading||!!error} onClick={()=>{if(compareFirst){setExpanded(v=>[...new Set([...v,'diff'])]);setDiffReviewed(true);requestAnimationFrame(()=>document.querySelector('[data-diff-section]')?.scrollIntoView({block:'center'}))}else setDecision(primary)}}>{compareFirst?'Посмотреть изменения':actionNames[primary.kind]}</Button></section>}
    {(snapshot.row.requests.length>1)&&<details className="notice"><summary>Остальные обращения ({snapshot.row.requests.length-1})</summary>{snapshot.row.requests.slice(1).map(r=><div key={r.id}><p>{decisionReason(r)}</p><Button variant="outline" disabled={editing||stale||loading||!!error} onClick={()=>setDecision(r)}>{actionNames[r.kind]}</Button></div>)}</details>}
    {editing&&session?<SpecEditor key={c.id} read={snapshot.read} session={session} onDirty={setDirty} onBusy={setWriteBusy} onSaved={saved} onCancel={()=>guard(()=>{setEditing(false);setDirty(false);requestAnimationFrame(()=>window.scrollTo(0,places.current.get(c.id)?.scroll??0))})} initialField={initialField}/>:<NativeDocument read={snapshot.read} row={snapshot.row} expanded={expanded} onExpanded={setExpanded} onEdit={snapshot.row.editable&&!loading?beginEdit:undefined}/>}
    {decision&&session&&<DecisionDialog onBusy={setWriteBusy} request={decision} read={snapshot.read} session={session} onClose={()=>setDecision(null)} onSaved={saved}/>}
   </article>:<div className="empty-state"><h1>{loading?'Открываем спецификации…':error?'Проект недоступен':!projection?.changes.length?'Каталог пока пуст':!rows.length?'В этом разделе пока нет спецификаций':'Выберите спецификацию'}</h1>{!loading&&!error&&!projection?.changes.length&&<p>Когда в проекте появятся поддерживаемые спецификации, они будут доступны здесь.</p>}</div>}
  </main>
  <Dialog open={!!pending} onOpenChange={open=>{if(!open)setPending(null)}}><DialogContent><DialogHeader><DialogTitle>Отменить несохранённые правки?</DialogTitle><DialogDescription>Введённый текст будет потерян. Можно продолжить редактирование.</DialogDescription></DialogHeader><DialogFooter><Button variant="outline" onClick={()=>setPending(null)}>Продолжить редактирование</Button><Button onClick={()=>{const action=pending;setPending(null);setDirty(false);action?.()}}>Отменить правки</Button></DialogFooter></DialogContent></Dialog>
 </div>;
}
