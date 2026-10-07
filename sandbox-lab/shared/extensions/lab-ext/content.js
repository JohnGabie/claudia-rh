// lab-ext: marcador visivel para provar que a extensao foi carregada.
(function () {
  if (document.getElementById("lab-ext-ok")) return;
  var b = document.createElement("div");
  b.id = "lab-ext-ok";
  b.textContent = "LAB-EXT OK";
  b.setAttribute("style", [
    "position:fixed", "top:0", "right:0", "z-index:2147483647",
    "background:#16a34a", "color:#fff", "font:bold 20px/1.2 Segoe UI,Arial,sans-serif",
    "padding:10px 18px", "border-bottom-left-radius:8px", "box-shadow:0 2px 8px rgba(0,0,0,.35)"
  ].join(";"));
  (document.body || document.documentElement).appendChild(b);
  if (document.title.indexOf("[LAB] ") !== 0) document.title = "[LAB] " + document.title;
})();
