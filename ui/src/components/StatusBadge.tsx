import type {QueueCategory} from '@/api/client';
import {Plus,Pencil,Minus} from 'lucide-react';

type GroupStatus='add'|'modify'|'remove'|'done'|'pending'|'passed'|'failed'|'skipped'|'go'|'no_go';
const labels:Record<QueueCategory|GroupStatus,string>={
 needs_decision:'Нужно решение',in_work:'В работе',completed:'Результат принят',
 cancelled:'Отменена',unknown:'Не удалось проверить',
 add:'Добавить',modify:'Изменить',remove:'Удалить',done:'Выполнено',pending:'Ожидает выполнения',
 passed:'Пройдено',failed:'Не пройдено',skipped:'Пропущено',go:'Одобрено',no_go:'Нужна доработка',
};
const tones:Record<GroupStatus,QueueCategory>={add:'completed',modify:'in_work',remove:'unknown',done:'completed',pending:'cancelled',passed:'completed',failed:'unknown',skipped:'cancelled',go:'completed',no_go:'unknown'};

export function StatusBadge({status}:{status:QueueCategory|GroupStatus}){
 if(status==='add'||status==='modify'||status==='remove'){
  const Icon={add:Plus,modify:Pencil,remove:Minus}[status];
  return <span className={`status-badge status-badge--${tones[status]} operation-icon`} role="img" aria-label={labels[status]} title={labels[status]}><Icon size={14} strokeWidth={2} aria-hidden="true"/></span>;
 }
 return <span className={`status-badge status-badge--${tones[status as GroupStatus]??status}`}>{labels[status]}</span>;
}
