/** The view consumes canonical fields, never an AI synopsis. Backend wiring is HG-0055/56. */
export interface Scenario {id:string;given:string;when:string;then:string;verification:string}
export interface Requirement {id:string;title:string;statement:string;scenarios:Scenario[]}
export interface SpecDocument {id:string;title:string;goal:string;rationale:string;scope:string;requirements:Requirement[];tasks:{id:string;description:string;done:boolean}[];dependencies:{id:string;title:string}[]}
export type Category='all'|'decision'|'work'|'completed';
export type ScreenState='approval'|'question'|'acceptance'|'reading'|'long'|'editing'|'conflict'|'validation'|'stale'|'loading'|'empty'|'search-empty'|'unsupported'|'corrupt'|'offline'|'integrated'|'diff';
