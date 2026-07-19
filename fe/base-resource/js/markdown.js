/* =========================================================================
   base-resource/js/markdown.js  —  md highlight (dark code-window) + preview render
   hl()      → spans aligned to components.css .code-window (.h/.s/.c/.lk)
   render()  → semantic HTML for cream preview (.wl coral links)
   <script src="../../base-resource/js/markdown.js"></script>  →  window.MD.*
   ========================================================================= */
(function(){
  function esc(s){return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');}
  function nameOf(path){const m=path.match(/([^\/]+)\.md$/);return m?m[1]:path;}

  /* hl — for dark .code-window editor source */
  function hl(md){
    md=esc(md);
    md=md.replace(/```[\s\S]*?```/g,m=>'<span class="cb">'+m+'</span>');        // fence → muted
    md=md.replace(/^#{1,6}\s.*$/gm,m=>'<span class="h">'+m+'</span>');          // heading → on-dark bold
    md=md.replace(/\*\*([^*]+)\*\*/g,'<b>$1</b>');
    md=md.replace(/`[^`]+`/g,m=>'<span class="ic">'+m+'</span>');               // inline code → amber
    md=md.replace(/\[\.knowledges[^\]]+\]/g,m=>'<span class="lk">'+m+'</span>'); // wikilink → coral
    md=md.replace(/^([-*])\s/gm,'<span class="lm">$1</span> ');                // list marker → muted
    return md;
  }

  /* render — semantic HTML for cream preview */
  function render(md){
    const parts=md.split(/(```[\s\S]*?```)/g);
    return parts.map(part=>{
      if(/^```[\s\S]*```$/.test(part)){const inner=part.replace(/^```\w*\n?/,'').replace(/```$/,'');return '<pre>'+esc(inner)+'</pre>';}
      let h=esc(part);
      h=h.replace(/^#{2}\s.*$/gm,m=>'\n<h2>'+esc(m.replace(/^#{2}\s/,''))+'</h2>');
      h=h.replace(/\*\*([^*]+)\*\*/g,'<strong>$1</strong>');
      h=h.replace(/`([^`]+)`/g,'<code>$1</code>');
      h=h.replace(/\[\.knowledges\/([^\]|]+)(?:\|([^\]]+))?\]/g,(m,p,r)=>'<a class="wl" data-rel="'+esc(r||'链接')+'">'+esc(nameOf(p))+'</a>');
      h=h.replace(/^([-*])\s.*$/gm,m=>'<li>'+esc(m.replace(/^[-*]\s/,''))+'</li>');
      h=h.replace(/(<li>[\s\S]*?<\/li>)(?!\s*<li>)/g,m=>'<ul>'+m+'</ul>');
      h=h.replace(/<\/li>\s*<li>/g,'</li><li>');
      h=h.replace(/\n{2,}/g,'\n\n');
      h=h.split(/\n{2}/).map(b=>(/^\s*<(h2|ul|pre)/.test(b)?b:'<p>'+b.replace(/\n/g,'<br>')+'</p>').join('\n');
      return h;
    }).join('\n');
  }

  window.MD={hl,render,esc,nameOf};
  /* bare aliases — existing call sites (hl(...), render(...)) work unchanged */
  window.hl=hl; window.render=render; window.esc=esc; window.nameOf=nameOf;
})();
