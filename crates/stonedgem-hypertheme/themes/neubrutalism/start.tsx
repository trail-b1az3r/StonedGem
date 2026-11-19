// Neubrutalism: the start page (Theme Studio). `theme` and `oynix` are globals.
export default function Start() {
  const tiles = ["accent", "surface_alt", "background", "text"];
  return (
    <main class="ht-start">
      <section class="ht-card hero">
        <h1>{theme.product}</h1>
        <p>Type an address or a search in the bar above.</p>
        <div class="swatches">
          {tiles.map((k) => <span class="ht-button" style={"background:" + theme.tokens[k]}>{k}</span>)}
        </div>
      </section>
      <p class="made">{theme.name}, made in Theme Studio</p>
    </main>
  );
}
