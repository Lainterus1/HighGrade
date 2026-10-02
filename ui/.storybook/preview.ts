import type {Preview} from '@storybook/react-vite';
import '../src/styles/theme.css';
const preview:Preview={initialGlobals:{a11y:{manual:true}},parameters:{layout:'fullscreen',options:{storySort:{order:['Начало','Основы','Компоненты','Спецификации','Сценарии']}},controls:{expanded:true},a11y:{test:'error'},backgrounds:{options:{warm:{name:'HighGrade',value:'#fffefa'}},default:'warm'}}};
export default preview;
