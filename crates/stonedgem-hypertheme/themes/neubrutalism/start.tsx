// Neubrutalism: the start page (Theme Studio). `theme` and `oynix` are globals.
export default function Start() {
  // Each swatch: [token, its text colour].
  const tiles = [["accent", "on_accent"], ["surface_alt", "text"], ["background", "text"], ["text", "background"]];
  return (
    <main class="ht-start">
      <section class="ht-card hero">
        <h1>{theme.product}</h1>
        <p>Type an address or a search in the bar above.</p>
        <div class="swatches">
          {tiles.map(([k, fg]) => <span class="ht-button" style={"background:" + theme.tokens[k] + ";color:" + theme.tokens[fg]}>{k.replace("_", " ")}</span>)}
        </div>
      </section>
      <p class="made">{theme.name}, made in Theme Studio</p>
    </main>
  );
}
