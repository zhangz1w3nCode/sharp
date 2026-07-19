/* =========================================================================
   base-resource/js/ui.js  —  shared UI interactions (extracted from v2)
   toast · makeResizer · windowChrome (traffic lights + titlebar drag)
   <script src="../../base-resource/js/ui.js"></script>  →  window.UI.*
   ========================================================================= */
(function(){
  /* toast — dark-navy pill; auto-creates element if page lacks #toast */
  function toast(msg, ms){
    let t=document.getElementById('toast')||document.getElementById('ui-toast');
    if(!t){ t=document.createElement('div'); t.id='ui-toast'; t.className='toast'; document.body.appendChild(t); }
    t.textContent=msg; t.classList.add('show');
    clearTimeout(toast._t); toast._t=setTimeout(()=>t.classList.remove('show'), ms||1600);
  }

  /* makeResizer — draggable divider; pane = element whose width is resized */
  function makeResizer(el, pane, min, max){
    let sx=0, sw=0, drg=false;
    const mv=(e)=>{ if(!drg)return; const w=Math.max(min,Math.min(max,sw+(e.clientX-sx))); pane.style.width=w+'px'; };
    const up=()=>{ drg=false; document.body.style.userSelect=''; document.removeEventListener('mousemove',mv); document.removeEventListener('mouseup',up); };
    el.addEventListener('mousedown',(e)=>{ e.preventDefault(); drg=true; sx=e.clientX; sw=pane.offsetWidth; document.body.style.userSelect='none'; document.addEventListener('mousemove',mv); document.addEventListener('mouseup',up); });
  }

  /* windowChrome — traffic lights + titlebar drag; reopen = restore button (optional) */
  function windowChrome(win, titlebar, reopen){
    const d={on:false,x:0,y:0,ox:0,oy:0};
    titlebar.addEventListener('mousedown',(e)=>{ if(e.target.closest('button,a,.lights,.resizer'))return; d.on=true; d.x=e.clientX; d.y=e.clientY; const m=win.style.transform.match(/translate\(([-\d.]+)px,\s*([-\d.]+)px\)/); d.ox=m?parseFloat(m[1]):0; d.oy=m?parseFloat(m[2]):0; document.body.style.userSelect='none'; });
    document.addEventListener('mousemove',(e)=>{ if(!d.on)return; win.style.transform=`translate(${d.ox+e.clientX-d.x}px, ${d.oy+e.clientY-d.y}px)`; });
    document.addEventListener('mouseup',()=>{ d.on=false; document.body.style.userSelect=''; });
    document.body.addEventListener('dblclick',(e)=>{ if(e.target===document.body) win.style.transform=''; });
    const red=win.querySelector('.light.red'), yel=win.querySelector('.light.yellow'), grn=win.querySelector('.light.green');
    if(red) red.addEventListener('click',()=>{ win.style.transform='scale(.94)'; win.style.opacity='0'; win.style.pointerEvents='none'; if(reopen) setTimeout(()=>reopen.classList.add('show'),180); });
    if(yel) yel.addEventListener('click',()=>{ win.style.transform='translateY(60px) scale(.92)'; win.style.opacity='0'; setTimeout(()=>{win.style.transform='';win.style.opacity='1';},600); });
    if(grn){ let mx=false; grn.addEventListener('click',()=>{ mx=!mx; win.classList.toggle('maxed',mx); }); }
    if(reopen) reopen.addEventListener('click',()=>{ reopen.classList.remove('show'); win.style.transform=''; win.style.opacity='1'; win.style.pointerEvents=''; });
  }

  function esc(s){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');}
  window.UI={ toast, makeResizer, windowChrome, esc };
  /* bare aliases — existing call sites work unchanged */
  window.toast=toast; window.makeResizer=makeResizer; window.windowChrome=windowChrome; window.esc=esc;
})();
