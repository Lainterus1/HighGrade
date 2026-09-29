import type {Meta,StoryObj} from '@storybook/react-vite';
import {StatusBadge} from './StatusBadge';
const meta={title:'Компоненты/Статусы',component:StatusBadge} satisfies Meta<typeof StatusBadge>;
export default meta;
type Story=StoryObj<typeof meta>;
export const Decision:Story={name:'Нужно решение',args:{status:'needs_decision'}};
export const Work:Story={name:'В работе',args:{status:'in_work'}};
export const Complete:Story={name:'Результат принят',args:{status:'completed'}};
export const Cancelled:Story={name:'Отменена',args:{status:'cancelled'}};
export const Unknown:Story={name:'Не удалось проверить',args:{status:'unknown'}};

export const Add:Story={name:'Добавить',args:{status:'add'}};
export const Modify:Story={name:'Изменить',args:{status:'modify'}};
export const Remove:Story={name:'Удалить',args:{status:'remove'}};
export const Done:Story={name:'Выполнено',args:{status:'done'}};
export const Passed:Story={name:'Пройдено',args:{status:'passed'}};
export const Failed:Story={name:'Не пройдено',args:{status:'failed'}};
