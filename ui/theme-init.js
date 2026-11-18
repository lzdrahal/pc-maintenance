(function () {
  try {
    var stored = localStorage.getItem("smt-color-scheme");
    if (stored === "light" || stored === "dark") {
      var meta = document.querySelector('meta[name="color-scheme"]');
      if (meta) {
        meta.content = stored;
      }
    }
  } catch (_err) {
    // Ignore storage access errors and keep default light dark system scheme.
  }
})();
