"use strict";
const form = document.querySelector("#generator");
const generate = document.querySelector("#generate");
const error = document.querySelector("#error");
const numbers = document.querySelector("#numbers");
const details = document.querySelector("#details");
const copy = document.querySelector("#copy");
const copyStatus = document.querySelector("#copy-status");
let latest = [];

form.addEventListener("submit", async (event) => {
  event.preventDefault();
  if (generate.disabled) return;
  error.hidden = true;
  copyStatus.textContent = "";
  const fields = new FormData(form);
  const params = new URLSearchParams({
    min: fields.get("min"), max: fields.get("max"), count: fields.get("count"),
    unique: String(fields.has("unique"))
  });
  generate.disabled = true;
  document.querySelector(".results").setAttribute("aria-busy", "true");
  try {
    const response = await fetch(`/api/generate?${params}`, { cache: "no-store", headers: { Accept: "application/json" } });
    if (!(response.headers.get("content-type") || "").includes("application/json")) {
      throw new Error("Your LOKI session may have expired. Reload this page to sign in.");
    }
    const data = await response.json();
    if (!response.ok) throw new Error(data.error || "Could not generate numbers.");
    if (!Array.isArray(data.numbers) || !data.numbers.every(Number.isSafeInteger)) {
      throw new Error("The server returned an invalid result. Please try again.");
    }
    latest = data.numbers;
    numbers.replaceChildren(...latest.map((value) => {
      const item = document.createElement("span");
      item.className = "number";
      item.textContent = String(value);
      return item;
    }));
    details.textContent = `${latest.length} ${latest.length === 1 ? "number" : "numbers"} · ${data.options.min} to ${data.options.max}${data.options.unique ? " · No repeats" : " · Repeats allowed"}`;
    copy.hidden = false;
  } catch (cause) {
    error.textContent = cause instanceof TypeError ? "Could not reach LOKI. Check your connection and try again." : cause.message;
    error.hidden = false;
  } finally {
    generate.disabled = false;
    document.querySelector(".results").setAttribute("aria-busy", "false");
  }
});

document.querySelectorAll("[data-min]").forEach((button) => {
  button.addEventListener("click", () => {
    if (generate.disabled) return;
    form.elements.min.value = button.dataset.min;
    form.elements.max.value = button.dataset.max;
    form.elements.count.value = "1";
    form.elements.unique.checked = false;
    form.requestSubmit();
  });
});

copy.addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(latest.join(", "));
    copyStatus.textContent = "Copied to clipboard.";
  } catch {
    const selection = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(numbers);
    selection.removeAllRanges();
    selection.addRange(range);
    copyStatus.textContent = "Results selected. Press Ctrl+C or use your browser’s Copy command.";
  }
});
