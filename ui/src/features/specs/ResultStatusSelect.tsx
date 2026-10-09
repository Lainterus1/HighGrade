export function ResultStatusSelect({value,onChange,disabled=false}:{value:'accepted'|'needs_changes';onChange:(value:'accepted'|'needs_changes')=>void;disabled?:boolean}){
 return <><label htmlFor="result-status">Статус результата</label><select id="result-status" className="result-status-select" value={value} onChange={event=>onChange(event.target.value as 'accepted'|'needs_changes')} disabled={disabled}><option value="accepted">Результат принят</option><option value="needs_changes">Нужна доработка</option></select></>;
}
