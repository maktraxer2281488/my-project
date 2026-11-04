(() => {
  "use strict";

  const $ = (s, root = document) => root.querySelector(s);
  const $$ = (s, root = document) => [...root.querySelectorAll(s)];
  const fmt = (n) => Math.round(n).toLocaleString("ru-RU");
  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  const SVG_NS = "http://www.w3.org/2000/svg";

  /* ---------- Tire: tread blocks & rim spokes ---------- */
  const tread = $(".tread");
  if (tread) {
    const count = 48;
    for (let i = 0; i < count; i++) {
      const a = (360 / count) * i;
      const r = document.createElementNS(SVG_NS, "rect");
      const long = i % 2 === 0;
      r.setAttribute("x", "196");
      r.setAttribute("y", long ? "12" : "18");
      r.setAttribute("width", "8");
      r.setAttribute("height", long ? "30" : "22");
      r.setAttribute("rx", "2");
      r.setAttribute("transform", `rotate(${a} 200 200) skewX(-14)`);
      tread.appendChild(r);
    }
  }
  const spokes = $(".spokes");
  if (spokes) {
    for (let i = 0; i < 5; i++) {
      const p = document.createElementNS(SVG_NS, "path");
      // twin-spoke window between hub and rim
      p.setAttribute("d", "M188 156 L176 104 Q200 96 224 104 L212 156 Q200 152 188 156Z");
      p.setAttribute("transform", `rotate(${i * 72 + 36} 200 200)`);
      spokes.appendChild(p);
    }
  }

  /* ---------- Header ---------- */
  const header = $("#header");
  const onScroll = () => header.classList.toggle("is-scrolled", window.scrollY > 10);
  onScroll();
  window.addEventListener("scroll", onScroll, { passive: true });

  const burger = $("#burger");
  const nav = $("#nav");
  const closeNav = () => {
    nav.classList.remove("is-open");
    burger.setAttribute("aria-expanded", "false");
    burger.setAttribute("aria-label", "Открыть меню");
  };
  burger.addEventListener("click", () => {
    const open = nav.classList.toggle("is-open");
    burger.setAttribute("aria-expanded", String(open));
    burger.setAttribute("aria-label", open ? "Закрыть меню" : "Открыть меню");
  });
  $$("a", nav).forEach((a) => a.addEventListener("click", closeNav));
  document.addEventListener("keydown", (e) => e.key === "Escape" && closeNav());

  /* ---------- Reveal on scroll ---------- */
  const groups = new Map();
  $$(".reveal").forEach((el) => {
    const parent = el.parentElement;
    const idx = groups.get(parent) || 0;
    el.style.setProperty("--d", `${Math.min(idx, 6) * 0.08}s`);
    groups.set(parent, idx + 1);
  });
  if ("IntersectionObserver" in window && !reduceMotion) {
    const io = new IntersectionObserver(
      (entries) => entries.forEach((en) => {
        if (en.isIntersecting) { en.target.classList.add("is-in"); io.unobserve(en.target); }
      }),
      { threshold: 0.12, rootMargin: "0px 0px -40px 0px" }
    );
    $$(".reveal").forEach((el) => io.observe(el));
  } else {
    $$(".reveal").forEach((el) => el.classList.add("is-in"));
  }

  /* ---------- Counters ---------- */
  const animateCount = (el) => {
    const target = parseFloat(el.dataset.count);
    const decimals = +(el.dataset.decimals || 0);
    const suffix = el.dataset.suffix || "";
    const render = (v) => {
      el.textContent = (decimals ? v.toFixed(decimals).replace(".", ",") : fmt(v)) + suffix;
    };
    if (reduceMotion) return render(target);
    const dur = 1600;
    const start = performance.now();
    const tick = (now) => {
      const t = Math.min((now - start) / dur, 1);
      const eased = 1 - Math.pow(1 - t, 4);
      render(target * eased);
      if (t < 1) requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  };
  const counters = $$("[data-count]");
  if ("IntersectionObserver" in window) {
    const cio = new IntersectionObserver((entries) => entries.forEach((en) => {
      if (en.isIntersecting) { animateCount(en.target); cio.unobserve(en.target); }
    }), { threshold: 0.6 });
    counters.forEach((c) => cio.observe(c));
  } else counters.forEach(animateCount);

  /* ---------- Card spotlight ---------- */
  $$(".service").forEach((card) => {
    card.addEventListener("pointermove", (e) => {
      const r = card.getBoundingClientRect();
      card.style.setProperty("--mx", `${e.clientX - r.left}px`);
      card.style.setProperty("--my", `${e.clientY - r.top}px`);
    });
  });

  /* ---------- Calculator ---------- */
  const radius = $("#radius");
  const radiusOut = $("#radiusOut");
  const totalEl = $("#total");
  const durationEl = $("#duration");
  const runflat = $("#runflat");
  const PRICES = {
    // per wheel: base + step * (R - 13); minutes per wheel
    mount:   { base: 150, step: 20, min: 4,  scalable: true },
    tire:    { base: 200, step: 45, min: 5,  scalable: true },
    balance: { base: 200, step: 30, min: 4,  scalable: true },
    wash:    { base: 100, step: 10, min: 2,  scalable: false },
    nitro:   { base: 50,  step: 0,  min: 1,  scalable: false },
  };
  let shownTotal = 0;
  let totalRaf;

  const tweenTotal = (to) => {
    cancelAnimationFrame(totalRaf);
    if (reduceMotion) { shownTotal = to; totalEl.textContent = fmt(to); return; }
    const from = shownTotal;
    const start = performance.now();
    const step = (now) => {
      const t = Math.min((now - start) / 450, 1);
      shownTotal = from + (to - from) * (1 - Math.pow(1 - t, 3));
      totalEl.textContent = fmt(shownTotal);
      if (t < 1) totalRaf = requestAnimationFrame(step);
    };
    totalRaf = requestAnimationFrame(step);
  };

  const calc = () => {
    const R = +radius.value;
    const car = +$('input[name="car"]:checked').value;
    const rf = runflat.checked ? 1.3 : 1;
    radiusOut.textContent = `R${R}`;
    const pct = ((R - +radius.min) / (+radius.max - +radius.min)) * 100;
    radius.style.setProperty("--p", `${pct}%`);

    let total = 0;
    let minutes = 5; // check-in & final inspection
    $$("#calcServices input").forEach((cb) => {
      const p = PRICES[cb.value];
      let perWheel = p.base + p.step * (R - 13);
      if (p.scalable) perWheel *= car * rf;
      const four = Math.round((perWheel * 4) / 10) * 10;
      $(`.check__price[data-for="${cb.value}"]`).textContent = `${fmt(four)} ₽`;
      if (cb.checked) { total += four; minutes += p.min * 4; }
    });
    tweenTotal(total);
    durationEl.textContent = Math.round(minutes / 5) * 5;
  };
  [radius, runflat, ...$$('input[name="car"]'), ...$$("#calcServices input")].forEach((el) =>
    el.addEventListener("input", calc)
  );
  calc();

  /* ---------- Booking: days & slots ---------- */
  const daysEl = $("#days");
  const slotsEl = $("#slots");
  const WD = ["вс", "пн", "вт", "ср", "чт", "пт", "сб"];
  const MONTHS = ["января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября", "октября", "ноября", "декабря"];
  const SLOTS = [];
  for (let h = 8; h < 22; h++) { SLOTS.push(`${h}:00`); SLOTS.push(`${h}:30`); }

  // deterministic "busy" pattern per date so it looks realistic
  const isBusy = (date, i) => {
    const seed = date.getFullYear() * 372 + date.getMonth() * 31 + date.getDate();
    const x = Math.sin(seed * 9.73 + i * 4.11) * 10000;
    return x - Math.floor(x) < 0.35;
  };
  const isPast = (date, slot) => {
    const now = new Date();
    if (date.toDateString() !== now.toDateString()) return false;
    const [h, m] = slot.split(":").map(Number);
    return h * 60 + m <= now.getHours() * 60 + now.getMinutes() + 30;
  };

  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const dates = Array.from({ length: 14 }, (_, i) => {
    const d = new Date(today);
    d.setDate(d.getDate() + i);
    return d;
  });

  let selectedDate = dates[0];
  const renderSlots = () => {
    slotsEl.innerHTML = "";
    let firstFree = null;
    SLOTS.forEach((s, i) => {
      const disabled = isBusy(selectedDate, i) || isPast(selectedDate, s);
      const label = document.createElement("label");
      label.className = "slot";
      label.innerHTML = `<input type="radio" name="slot" value="${s}" ${disabled ? "disabled" : ""}/><span>${s}</span>`;
      slotsEl.appendChild(label);
      if (!disabled && !firstFree) firstFree = label.querySelector("input");
    });
    if (firstFree) firstFree.checked = true;
    return firstFree;
  };

  dates.forEach((d, i) => {
    const label = document.createElement("label");
    label.className = "day";
    const name = i === 0 ? "сег" : i === 1 ? "завт" : WD[d.getDay()];
    label.innerHTML = `<input type="radio" name="day" value="${i}" ${i === 0 ? "checked" : ""} aria-label="${d.getDate()} ${MONTHS[d.getMonth()]}"/><span><small>${name}</small><b>${d.getDate()}</b></span>`;
    label.querySelector("input").addEventListener("change", () => {
      selectedDate = dates[i];
      renderSlots();
    });
    daysEl.appendChild(label);
  });

  // If today is fully booked, jump to the next day with a free slot
  let first = renderSlots();
  for (let i = 1; !first && i < dates.length; i++) {
    selectedDate = dates[i];
    daysEl.querySelectorAll("input")[i].checked = true;
    first = renderSlots();
  }
  const nextSlot = $("#nextSlot");
  if (first && nextSlot) {
    const di = dates.indexOf(selectedDate);
    const when = di === 0 ? "сегодня" : di === 1 ? "завтра" : `${selectedDate.getDate()} ${MONTHS[selectedDate.getMonth()]}`;
    nextSlot.textContent = `${when} в ${first.value}`;
  }
  const freeToday = SLOTS.filter((s, i) => !isBusy(dates[0], i) && !isPast(dates[0], s)).length;
  const pill = $(".pill");
  if (pill) {
    pill.lastChild.textContent = freeToday
      ? `Сегодня свободно ${freeToday} ${plural(freeToday, ["окно", "окна", "окон"])}`
      : "Запись на завтра открыта";
  }

  function plural(n, forms) {
    const m10 = n % 10, m100 = n % 100;
    if (m10 === 1 && m100 !== 11) return forms[0];
    if (m10 >= 2 && m10 <= 4 && (m100 < 10 || m100 >= 20)) return forms[1];
    return forms[2];
  }

  /* ---------- Phone mask ---------- */
  const phone = $('input[name="phone"]');
  phone.addEventListener("input", () => {
    let d = phone.value.replace(/\D/g, "");
    if (!d) { phone.value = ""; return; }
    if (d[0] === "8") d = "7" + d.slice(1);
    if (d[0] !== "7") d = "7" + d;
    d = d.slice(0, 11);
    let out = "+7";
    if (d.length > 1) out += ` (${d.slice(1, 4)}`;
    if (d.length >= 4) out += ")";
    if (d.length > 4) out += ` ${d.slice(4, 7)}`;
    if (d.length > 7) out += `-${d.slice(7, 9)}`;
    if (d.length > 9) out += `-${d.slice(9, 11)}`;
    phone.value = out;
  });

  /* ---------- Calculator → booking ---------- */
  $("#calcBook").addEventListener("click", () => {
    $("#serviceSelect").value = "Сезонная переобувка";
  });
  $$("#storage .btn").forEach((b) => b.addEventListener("click", () => {
    $("#serviceSelect").value = "Сезонное хранение";
  }));

  /* ---------- Submit ---------- */
  const form = $("#bookingForm");
  const toast = $("#toast");
  let toastTimer;
  const showToast = (title, text) => {
    $("#toastTitle").textContent = title;
    $("#toastText").textContent = text;
    toast.classList.add("is-visible");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => toast.classList.remove("is-visible"), 5000);
  };

  form.addEventListener("submit", (e) => {
    e.preventDefault();
    const name = form.elements.name;
    const valid = {
      name: name.value.trim().length >= 2,
      phone: phone.value.replace(/\D/g, "").length === 11,
    };
    name.classList.toggle("is-invalid", !valid.name);
    phone.classList.toggle("is-invalid", !valid.phone);
    if (!valid.name) return name.focus();
    if (!valid.phone) return phone.focus();

    const slot = $('input[name="slot"]:checked');
    if (!slot) return showToast("Нет свободного времени", "Выберите другой день");

    const di = +$('input[name="day"]:checked').value;
    const d = dates[di];
    const when = di === 0 ? "сегодня" : di === 1 ? "завтра" : `${d.getDate()} ${MONTHS[d.getMonth()]}`;
    showToast(`${name.value.trim()}, вы записаны!`, `${form.elements.service.value} — ${when} в ${slot.value}`);

    slot.disabled = true;
    slot.checked = false;
    const next = $('input[name="slot"]:not(:disabled)');
    if (next) next.checked = true;
    form.reset();
    $$('input[name="day"]')[di].checked = true;
    if (next) next.checked = true;
  });
  [form.elements.name, phone].forEach((el) =>
    el.addEventListener("input", () => el.classList.remove("is-invalid"))
  );

  /* ---------- Misc ---------- */
  $("#year").textContent = new Date().getFullYear();
})();
