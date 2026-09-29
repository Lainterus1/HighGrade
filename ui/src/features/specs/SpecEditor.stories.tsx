import {useRef,useState} from 'react';
import type {Meta,StoryObj} from '@storybook/react-vite';
import type {ReadResult} from '@/api/client';
import {SpecEditor} from './SpecEditor';
import {demoSpec} from './fixtures';
const example:ReadResult={store_sha256:'demo',local_sha256:'demo',change_sha256:'demo',inputs_sha256:'demo',links:{},change:{id:'HG-DEMO-01',title:demoSpec.title,goal:demoSpec.goal,rationale:demoSpec.rationale,scope:demoSpec.scope,questions:[],operations:demoSpec.requirements.map(requirement=>({action:'add',requirement})),tasks:demoSpec.tasks,depends_on:[],related_to:[],tags:[],checks:[],runs:[],acceptance:[],history:[],evidence:{},review:null,archived:false,abandoned_reason:null}};
function Form(){const current=useRef(structuredClone(example));const [message,setMessage]=useState('');return <main className="document"><span className="demo-pill">Демонстрационные данные · без HTTP</span><h1>Редактирование полей</h1>{message&&<p role="status">{message}</p>}<SpecEditor read={example} session={{api_version:'1',token:'demo',project:{name:'Демонстрация',root:'Демонстрация'}}} onDirty={()=>{}} onBusy={()=>{}} onSaved={async()=>{setMessage('Сохранено только в памяти демонстрации. Файлы проекта не изменялись.')}} onCancel={()=>setMessage('Отмена показана; демонстрация остаётся открытой.')} transport={{read:async()=>current.current,write:async<T,>(_path:string,init?:RequestInit)=>{current.current={...current.current,change:{...current.current.change,...JSON.parse(String(init?.body)).input}};return {} as T}}}/></main>}
const meta={title:'Спецификации/Редактор полей',component:Form,parameters:{docs:{description:{component:'Производственная форма. Все операции этой истории подменены изолированной памятью; сетевых записей нет.'}}}} satisfies Meta<typeof Form>;
export default meta;type Story=StoryObj<typeof meta>;
export const Editing:Story={name:'Поля и валидация'};
