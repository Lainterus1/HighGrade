import type { SVGProps } from 'react';
import { Search, Pencil, Folder, ChevronRight, Menu, ArrowLeft, Info, ListFilter, CircleAlert, WifiOff, FileQuestion } from 'lucide-react';
export { Search, Pencil, Folder, ChevronRight, Menu, ArrowLeft, Info, ListFilter, CircleAlert, WifiOff, FileQuestion };
export const iconSources = [
  {name:'Все спецификации',source:'files',change:'Асимметричная пара листов, спокойные отступы и укороченные строки.'},
  {name:'Нужно решение',source:'message-square',change:'Открытый нижний угол и две строки вместо декоративных точек.'},
  {name:'В работе',source:'clock-3',change:'Разомкнутый контур часов и короткая стрелка подчёркивают продолжение.'},
  {name:'Завершены',source:'circle-check',change:'Открытый контур согласован с часами, галочка вынесена к правому краю.'},
];
export type IconName='all'|'decision'|'work'|'completed';
export function GradeIcon({name,size=20,...props}:SVGProps<SVGSVGElement>&{name:IconName;size?:number}) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" {...props}>
    {name==='all'&&<><path d="M7 3H4a1 1 0 0 0-1 1v13a2 2 0 0 0 2 2"/><path d="M10 5h7l4 4v11a1 1 0 0 1-1 1H9a1 1 0 0 1-1-1V7a2 2 0 0 1 2-2Z"/><path d="M16 5v5h5M12 14h5M12 17h3"/></>}
    {name==='decision'&&<><path d="M9 19 4 22V5a2 2 0 0 1 2-2h14a1 1 0 0 1 1 1v13a2 2 0 0 1-2 2h-6"/><path d="M8 8h9M8 12h6"/></>}
    {name==='work'&&<><path d="M20.3 7.5A9 9 0 1 0 21 15"/><path d="M12 7v5l4 2M20 3v5h-5"/></>}
    {name==='completed'&&<><path d="M20.8 13.8A9 9 0 1 1 15 3.5"/><path d="m8 11 4 4 9-10"/></>}
  </svg>;
}
