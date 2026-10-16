import assert from 'node:assert/strict';
import fs from 'node:fs';
import test from 'node:test';
class Target {
    listeners = new Map();
    addEventListener(kind, fn) { const set = this.listeners.get(kind) ?? new Set(); set.add(fn); this.listeners.set(kind, set); }
    removeEventListener(kind, fn) { this.listeners.get(kind)?.delete(fn); }
    emit(kind, extra = {}) { for (const fn of [...this.listeners.get(kind) ?? []]) fn({button:0,clientX:100,clientY:100,preventDefault(){},...extra}); }
    count() { return [...this.listeners.values()].reduce((n,set)=>n+set.size,0); }
}
const script = fs.readFileSync('crates/inputforge-app/assets/instrument_pointer.js','utf8');
function fixture() {
    const window = new Target(); const document = new Target(); const messages = [];
    let plot = new Target(); plot.getBoundingClientRect=()=>({left:0,top:0,right:200,bottom:200,width:200,height:200});
    document.getElementById = () => plot;
    const mount = owner => new Function('window','document','dioxus',script.replaceAll('__PLOT_ID__',"'plot'").replaceAll('__OWNER__',`'${owner}'`))(window,document,{send:m=>messages.push(m),recv:()=>new Promise(()=>{})});
    return {window,document,messages,mount,get plot(){return plot;},replace(){plot=new Target();plot.getBoundingClientRect=()=>({left:10,top:20,right:210,bottom:220,width:200,height:200});}};
}
for (const cancel of ['blur','pointercancel','lostpointercapture','Escape']) test(`${cancel} cancels without committing and cleanup removes listeners`, () => {
    const f=fixture();f.mount('owner');f.plot.emit('mousedown');
    if(cancel==='blur')f.window.emit('blur');
    else if(cancel==='Escape')f.document.emit('keydown',{key:'Escape'});
    else (cancel==='lostpointercapture'?f.plot:f.document).emit(cancel);
    f.document.emit('mouseup');
    assert.deepEqual(f.messages.map(m=>m.kind),['down','cancel']);
    f.window.__inputforgeInstruments.get('owner')();
    assert.equal(f.window.count()+f.document.count()+f.plot.count(),0);
});
test('duplicate owner installs once and a fresh owner remounts cleanly', () => {
    const f=fixture();f.mount('owner');f.mount('owner');
    f.plot.emit('mousedown');f.document.emit('mousemove',{clientX:300});f.document.emit('mouseup',{clientX:300});
    assert.deepEqual(f.messages.map(m=>m.kind),['down','move','up']);
    f.window.__inputforgeInstruments.get('owner')();f.document.emit('mouseup');
    assert.equal(f.window.__inputforgeInstruments.size,0);
    f.mount('fresh');f.plot.emit('mousedown');f.document.emit('mouseup');
    assert.deepEqual(f.messages.slice(3).map(m=>m.kind),['down','up']);
});
test('element identity rejects old events and measurements follow the current rect', () => {
    const f=fixture();f.mount('old');const old=f.plot;old.emit('mousedown');f.replace();
    f.document.emit('mousemove');f.document.emit('mouseup');old.emit('dblclick');
    assert.deepEqual(f.messages.map(m=>m.kind),['down']);
    assert.equal(f.window.count()+f.document.count()+old.count(),0);
    f.mount('new');f.plot.emit('mousedown');
    assert.deepEqual([f.messages.at(-1).rl,f.messages.at(-1).rt,f.messages.at(-1).rs],[10,20,200]);
    f.plot.emit('contextmenu');
    assert.deepEqual(f.messages.slice(-2).map(m=>m.kind),['up','ctx']);
});
test('a missing plot installs no listeners', () => {
    const f=fixture();f.document.getElementById=()=>null;f.mount('missing');
    assert.equal(f.window.count()+f.document.count()+f.plot.count(),0);
    assert.equal(f.window.__inputforgeInstruments,undefined);
});
