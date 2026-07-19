const oldRepository = "https://github.com/dreamlovesu32/chemcore";
const newRepository = "https://github.com/dreamlovesu32/chemsema";
const oldPages = "https://dreamlovesu32.github.io/chemcore/";
const newPages = "https://dreamlovesu32.github.io/chemsema/";

const normalize = (value) => value.replace(/\/+$/, "").toLowerCase();

async function checkRepositoryRedirect() {
  const response = await fetch(oldRepository, {
    redirect: "manual",
    headers: { "user-agent": "ChemSema redirect monitor" },
  });
  const location = response.headers.get("location") || "";
  if (response.status < 300 || response.status >= 400) {
    throw new Error(`repository returned HTTP ${response.status}, expected a redirect`);
  }
  if (normalize(new URL(location, oldRepository).href) !== normalize(newRepository)) {
    throw new Error(`repository redirected to ${location || "<missing location>"}`);
  }
}

async function checkPagesRedirect() {
  const response = await fetch(oldPages, {
    redirect: "follow",
    headers: { "user-agent": "ChemSema redirect monitor" },
  });
  if (!response.ok) {
    throw new Error(`legacy Pages path returned HTTP ${response.status}`);
  }
  const html = await response.text();
  if (!html.includes("data-chemsema-redirect") || !html.includes(newPages)) {
    throw new Error("legacy Pages path does not contain the maintained ChemSema redirect");
  }
}

if (process.env.CHEMSEMA_SKIP_REDIRECT_CHECK === "1") {
  console.log("Legacy redirect check skipped by CHEMSEMA_SKIP_REDIRECT_CHECK=1.");
  process.exit(0);
}

const checks = [
  ["GitHub repository", checkRepositoryRedirect],
  ["GitHub Pages", checkPagesRedirect],
];

let failed = false;
for (const [label, check] of checks) {
  try {
    await check();
    console.log(`${label}: OK`);
  } catch (error) {
    failed = true;
    console.error(`${label}: FAILED — ${error.message}`);
  }
}

if (failed) {
  process.exit(1);
}
