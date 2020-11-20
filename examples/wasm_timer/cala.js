const imports = { env: {
    navigator_userAgent_Len:()=>l(navigator.userAgent),
    navigator_userAgent_Ptr:(ptr)=>m(ptr),
    console_warn:(ptr,len)=>console.warn(f(ptr,len)),
    console_info:(ptr,len)=>console.info(f(ptr,len)),
    console_debug:(ptr,len)=>console.debug(f(ptr,len)),
    alert:(ptr,len)=>alert(f(ptr,len)),
} };
let wasm_filename = 'cala.wasm';
if (!('WebAssembly' in window)) {
    // Fallback asm.js
    // FIXME
    console.info("Using asm.js (fallback)...");
} else if (!('instantiateStreaming' in window.WebAssembly)) {
    console.info("Using non-streaming WASM (fallback)...");
    // Fallback to non-streaming
    fetch(wasm_filename)
        .then(response => response.arrayBuffer())
        .then(bytes => WebAssembly.instantiate(bytes, imports))
        .then(results => {
            results.instance.exports.exported_func();
        });
} else {
    console.info("Using streaming WASM (most efficient)...");
    // Most efficient solution (available & preferred on most browsers
    // except Safari)
    WebAssembly.instantiateStreaming(fetch(wasm_filename), imports)
        .then(obj => obj.instance.exports.exported_func());
}

var LINEAR_MEMORY = instance.exports.memory;
function f(ptr,len) {
    let buffer = new Uint16Array(LINEAR_MEMORY.buffer, ptr, len);
    let str= "";
    for(let i = 0; i < buffer.length; i += 1) {
        str += String.fromCharCode(buffer[i]);
    }
    return str;
}
function l(s) {
    t = new String(s);
    return t.length;
}
function m(ptr) {
    let buffer = new Uint16Array(LINEAR_MEMORY.buffer, ptr, t.length);
    for(let i = 0; i < t.length; i += 1) {
        buffer[i]=t.charCodeAt(i);
    }
}
// instance.exports.main();
