import {useState} from 'react';
import type {Meta,StoryObj} from '@storybook/react-vite';
import type {ReadResult,Row} from '@/api/client';
import {NativeDocument} from './NativeDocument';
import {demoSpec} from './fixtures';
const read:ReadResult={store_sha256:'demo',local_sha256:'demo',change_sha256:'demo',inputs_sha256:'demo',links:{depends_on:[{id:'HG-DEMO-00',reason:'Исходное правило'}]},change:{id:demoSpec.id,title:demoSpec.title,goal:demoSpec.goal,rationale:'',scope:demoSpec.scope,questions:['Как сохранить введённый текст?'],operations:[...demoSpec.requirements.map(requirement=>({action:'add' as const,requirement})),{action:'remove',id:'HG-DEMO-OLD',reason:'Прежнее правило заменено явным сохранением.'}],tasks:demoSpec.tasks,depends_on:[],related_to:[],tags:[],checks:[],runs:[],acceptance:[],history:[],evidence:{},review:null,archived:false,abandoned_reason:null}};
const row:Row={id:demoSpec.id,title:demoSpec.title,tags:[],created_at:0,category:'in_work',content_sha256:'demo',editable:true,technical_ready:false,human:'pending',requirements_agreement:'pending',primary_action:null,requests:[],history:[],diagnostic:null,changes_since_agreement:null};
function Example(){const [expanded,setExpanded]=useState<string[]>([]);return <main className="document spec-document"><span className="demo-pill">Демонстрационные данные</span><h1>Краткая карточка</h1><NativeDocument read={read} row={row} expanded={expanded} onExpanded={setExpanded}/></main>}
const meta={title:'Спецификации/Исходный документ',component:Example,parameters:{docs:{description:{component:'Тот же компонент отображает реальный API: цель, требования, границы, условия готовности и вопросы. В этой истории данные изолированы.'}}}} satisfies Meta<typeof Example>;
export default meta;type Story=StoryObj<typeof meta>;
export const Full:Story={name:'Компактная карточка'};
