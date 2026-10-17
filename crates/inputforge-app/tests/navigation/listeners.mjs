import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const handlers=new Set();const window={addEventListener(k,h,c){assert.equal(c,true);if(k==='keydown')handlers.add(h)},removeEventListener(k,h,c){if(k==='keydown')handlers.delete(h)}};
class Element{constructor(kind='Outside',stage=null,parent=null){this.kind=kind;this.stage=stage;this.parentElement=parent;this.dataset={stageId:stage,navigationContext:'profile:mapping',navigationActions:'rendered actions'};}closest(s){if(s==='.if-editor')return this.kind==='Outside'?null:this;if(s.includes('.if-stage'))return this.stage===null?null:this;if(s.includes('input'))return this.kind==='Input'?this:null;return null}matches(s){return Boolean(this.closest(s))}getBoundingClientRect(){return {left:4,bottom:9}}}
const document={activeElement:new Element('Editor'),modal:false,querySelector(s){return this.modal?{}:null}};
function install(name,key){const messages=[];let recv;const dioxus={send(v){messages.push(structuredClone(v))},recv(){return new Promise(r=>recv=r)}};vm.runInNewContext(readFileSync(`crates/inputforge-app/assets/${name}`,'utf8').replaceAll('__LISTENER_KEY__',key),{window,document,Element,dioxus});return{messages,stop(){window[key]?.();delete window[key];recv('__shutdown__')}}}
function fire(key,opts={}){const ev={key,target:document.activeElement,ctrlKey:false,metaKey:false,shiftKey:false,altKey:false,defaultPrevented:false,preventDefault(){this.defaultPrevented=true},stopPropagation(){},stopImmediatePropagation(){this.immediate=true},...opts};for(const h of [...handlers]){h(ev);if(ev.immediate)break}return ev.defaultPrevented}

const editor=install('editor_shortcuts.js','editor');
assert.equal(fire('z',{ctrlKey:true}),true);assert.equal(editor.messages.length,1);
document.activeElement=new Element('Input');assert.equal(fire('z',{ctrlKey:true}),false);assert.equal(editor.messages.length,1);
document.modal=true;document.activeElement=new Element('Editor');fire('y',{ctrlKey:true});assert.equal(editor.messages.length,1);document.modal=false;
window.__ifSortableDrag={};fire('y',{ctrlKey:true});assert.equal(editor.messages.length,1);delete window.__ifSortableDrag;
document.activeElement=new Element('Stage','0.T.1');fire('ArrowUp',{altKey:true});assert.equal(editor.messages.at(-1)[5],'0.T.1');assert.equal(editor.messages.at(-1)[9],'rendered actions');
editor.stop();assert.equal(handlers.size,0);const fresh=install('editor_shortcuts.js','fresh');await new Promise(r=>setImmediate(r));assert.equal(handlers.size,1);fresh.stop();
const rail=install('rail_shortcuts.js','rail');document.activeElement=new Element('Editor');fire('ArrowDown');assert.equal(rail.messages.length,0);fire('f',{ctrlKey:true});assert.equal(rail.messages.length,1);rail.messages.length=0;document.activeElement=new Element();fire('ArrowDown');assert.equal(rail.messages.length,1);document.modal=true;fire('Escape');assert.equal(rail.messages.length,1);document.modal=false;window.__ifSortableDrag={};fire('ArrowDown');assert.equal(rail.messages.length,1);delete window.__ifSortableDrag;rail.stop();assert.equal(handlers.size,0);
process.stdout.write('Navigation listener scope, drag, nested path and lifecycle checks passed.\n');

const capture=install('keyboard_capture_listener.js','capture');
const capturedEditor=install('editor_shortcuts.js','captured-editor');
document.activeElement=new Element('Stage','0.T.1');
window.capture.arm(8);fire('z',{ctrlKey:true});assert.equal(capturedEditor.messages.length,0);assert.equal(capture.messages.length,1);
window.capture.disarm();fire('z',{ctrlKey:true});assert.equal(capturedEditor.messages.length,1);
capturedEditor.stop();capture.stop();await new Promise(r=>setImmediate(r));assert.equal(handlers.size,0);
