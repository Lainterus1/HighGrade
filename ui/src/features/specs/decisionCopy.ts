import type {Handoff} from '@/api/client';

// Only labelled technical references move out of the reading text. No summary is generated.
export function decisionReason(request:Handoff){
 if(request.kind!=='result')return request.reason;
 return request.reason
  .replace(/Проверенная редакция\s+[A-Za-z0-9][A-Za-z0-9._:-]*(?:[.;](?=\s|$)|$)\s*/gu,'')
  .replace(/(?:Итог\/проверки:\s*|Проверки и версия\s*—\s*)(?:docs|specs|target)\/[^\s;]+[;]?\s*/gu,'')
  .trim();
}
