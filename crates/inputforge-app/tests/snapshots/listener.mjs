// Execute snapshot focus policy together with the production capture interceptor.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';
const source=fs.readFileSync('crates/inputforge-app/assets/snapshot_shortcut.js','utf8');
const capture=fs.readFileSync('crates/inputforge-app/assets/keyboard_capture_listener.js','utf8');
const handlers=new Map();
const window={addEventListener(k,h,c){assert.equal(c,true);const s=handlers.get(k)??new Set();s.add(h);handlers.set(k,s);},removeEventListener(k,h,c){assert.equal(c,true);handlers.get(k)?.delete(h);}};
let modal=false,menu=false;
const element=(tag='BUTTON',selectors=[])=>({tagName:tag,isContentEditable:false,closest:s=>selectors.includes(s)?{}:null});
const document={activeElement:element(),querySelector:s=>s==='dialog[open]'?modal:menu};
function install(source,key){const sent=[];let resolve;vm.runInNewContext(`{${source.replaceAll('__LISTENER_KEY__',key).replaceAll('__GENERATION__','1')}}`,{window,document,dioxus:{send:v=>sent.push(v),recv:()=>new Promise(r=>{resolve=r;})}});return {sent,shutdown:()=>resolve('__shutdown__')};}
function fire(type,overrides={}){const e={target:window,key:'s',ctrlKey:true,metaKey:false,shiftKey:false,altKey:false,repeat:false,isComposing:false,defaultPrevented:false,preventDefault(){this.defaultPrevented=true;},stopPropagation(){},stopImmediatePropagation(){this.immediate=true;},...overrides};for(const h of [...handlers.get(type)??[]]){h(e);if(e.immediate)break;}return e;}
const cap=install(capture,'if-keyboard-capture-test');
const a=install(source,'snapshot-a');
window['snapshot-a'].arm(1);
assert.equal(fire('keydown').defaultPrevented,true);
assert.equal(a.sent.length,1);
for(const tag of ['INPUT','TEXTAREA','SELECT']){document.activeElement=element(tag);assert.equal(fire('keydown').defaultPrevented,false);}
document.activeElement={...element('DIV'),isContentEditable:true};fire('keydown');
for(const selector of ['.profile-row__rename','[data-inline-rename]','[role="menu"]:not([aria-hidden="true"])','[role="dialog"][aria-modal="true"]']){document.activeElement=element('BUTTON',[selector]);fire('keydown');}
document.activeElement=element();modal=true;fire('keydown');modal=false;menu=true;fire('keydown');menu=false;
for(const changes of [{repeat:true},{shiftKey:true},{altKey:true},{ctrlKey:false},{key:'q'},{isComposing:true},{defaultPrevented:true}])fire('keydown',changes);
assert.equal(a.sent.length,1,'editable/modal/repeat/intercepted keys must not dispatch');
fire('blur');fire('focus');fire('keydown');assert.equal(a.sent.length,1,'picker return must not activate stale focus');
for(const key of ['Control','Meta','Alt','Shift','AltGraph']) {fire('keydown',{key});fire('keydown');assert.equal(a.sent.length,1,'modifier then save must retain native return suppression');}
fire('pointerdown');fire('keydown',{key:'Control'});fire('keydown');assert.equal(a.sent.length,2);
fire('blur',{target:element('INPUT')});fire('keydown');assert.equal(a.sent.length,3,'ordinary element blur must not be mistaken for a native picker return');
window['if-keyboard-capture-test'].arm(7);fire('keydown');assert.equal(a.sent.length,3,'capture must win before the root shortcut');window['if-keyboard-capture-test'].disarm();
window['snapshot-a']();assert.equal(handlers.get('keydown').size,1,'synchronous cleanup leaves only capture');
fire('keydown');assert.equal(a.sent.length,3);
const b=install(source,'snapshot-b');window['snapshot-b'].arm(2);fire('keydown');assert.equal(b.sent.length,1,'fresh remount acts exactly once');
a.shutdown();await new Promise(r=>setImmediate(r));fire('keydown');assert.equal(b.sent.length,2,'late old shutdown must not remove newer owner');
window['snapshot-b']();window['if-keyboard-capture-test']();b.shutdown();cap.shutdown();await new Promise(r=>setImmediate(r));
assert.equal(handlers.get('keydown').size,0);
assert.equal(handlers.get('blur').size,0);
assert.equal(handlers.get('pointerdown').size,0);
console.log('snapshot shortcut focus, repeat, capture, picker return, cleanup and remount passed');
