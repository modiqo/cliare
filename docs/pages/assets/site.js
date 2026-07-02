const starTarget = document.querySelector("[data-github-stars]");

if (starTarget) {
  fetch("https://api.github.com/repos/modiqo/cliare", {
    headers: { Accept: "application/vnd.github+json" },
  })
    .then((response) => (response.ok ? response.json() : null))
    .then((repo) => {
      if (!repo || typeof repo.stargazers_count !== "number") {
        return;
      }
      starTarget.textContent = new Intl.NumberFormat("en", {
        notation: repo.stargazers_count >= 1000 ? "compact" : "standard",
        maximumFractionDigits: 1,
      }).format(repo.stargazers_count);
    })
    .catch(() => {});
}
