(() => {
  'use strict';

  const $ = (id) => document.getElementById(id);
  const preview = $('demo');
  const marker = $('edge-marker');
  const tabs = $('note-tabs');
  const sheet = $('note');
  const title = $('note-title');
  const body = $('note-body');
  const connector = $('connector');
  const checklist = $('checklist-control');
  const pin = $('pin-control');
  const panels = ['color', 'font', 'more'];
  const idleMilliseconds = 60_000;
  // Fill and accent values come from NoteColor in src/app.rs.
  const colors = {
    green: { name: 'Sage', fill: 'rgb(75% 97% 81%)', accent: '#6fae68' },
    yellow: { name: 'Amber', fill: 'rgb(98% 93% 74%)', accent: '#d98c2b' },
    blue: { name: 'Blue', fill: 'rgb(71% 85% 100%)', accent: '#5b8def' },
    pink: { name: 'Pink', fill: 'rgb(100% 81% 89%)', accent: '#e8679c' },
  };
  const samples = () => [
    { title: 'Before I forget', color: 'green', font: 'virgil', lines: [
      { text: 'Send the draft', checked: true },
      { text: 'Water the plants', checked: false },
      { text: 'Book the train', checked: false },
    ] },
    { title: 'Weekend', color: 'yellow', font: 'virgil', lines: [
      { text: 'A slow morning.', checked: null },
      { text: 'Coffee, a book, and nowhere to be.', checked: null },
      { text: 'Try the bakery on the corner', checked: false },
    ] },
    { title: 'A small idea', color: 'pink', font: 'virgil', lines: [
      { text: 'A photo of the same tree, every Sunday.', checked: null },
      { text: 'See what changes.', checked: null },
    ] },
  ];
  // All visitor content stays here. Refresh creates fresh sample notes.
  let notes = samples();
  let active = null;
  let currentLine = 0;
  let caret = 0;
  let alwaysOpen = false;
  let openPanel = null;
  let idleTimer;
  let leaveTimer;
  let collapseTimer;
  let lastActivity = 0;

  function announce(message) { $('demo-status').textContent = message; }
  function currentNote() { return active === null ? null : notes[active]; }

  function updateTabs() {
    [...tabs.children].forEach((tab, index) => {
      tab.firstChild.textContent = notes[index].title.trim() || 'Untitled';
      tab.setAttribute('aria-label', `Open ${notes[index].title.trim() || 'untitled note'}`);
      tab.setAttribute('aria-expanded', String(index === active));
      tab.style.setProperty('--tab-color', colors[notes[index].color].fill);
    });
  }

  notes.forEach((_, index) => {
    const tab = document.createElement('button');
    tab.className = 'note-tab';
    tab.append(document.createElement('span'));
    tab.setAttribute('aria-controls', 'note');
    tab.addEventListener('click', () => openNote(index));
    tab.addEventListener('pointerenter', () => {
      if (active === index) connector.classList.add('tab-hovered');
    });
    tab.addEventListener('pointerleave', () => connector.classList.remove('tab-hovered'));
    tab.addEventListener('focus', () => {
      if (active === index) connector.classList.add('tab-hovered');
    });
    tab.addEventListener('blur', () => connector.classList.remove('tab-hovered'));
    tabs.append(tab);
  });

  function revealTabs() {
    clearTimeout(leaveTimer);
    clearTimeout(collapseTimer);
    tabs.classList.remove('collapsing');
    tabs.hidden = false;
    tabs.inert = false;
    marker.hidden = true;
    marker.setAttribute('aria-expanded', 'true');
    preview.dataset.view = active === null ? 'tabs' : 'note';
  }

  function collapseTabs(restoreFocus = false) {
    if (active !== null) return;
    closePanel();
    clearTimeout(leaveTimer);
    clearTimeout(collapseTimer);
    preview.dataset.view = 'marker';
    marker.hidden = false;
    marker.setAttribute('aria-expanded', 'false');
    if (restoreFocus || tabs.contains(document.activeElement)) marker.focus({ preventScroll: true });
    tabs.inert = true;
    tabs.classList.add('collapsing');
    collapseTimer = setTimeout(() => { tabs.hidden = true; },
      window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 110);
  }

  function scheduleCollapse() {
    clearTimeout(leaveTimer);
    leaveTimer = setTimeout(() => {
      if (active === null && !tabs.matches(':hover') && !tabs.contains(document.activeElement)) collapseTabs();
    }, 200);
  }

  marker.addEventListener('pointerenter', (event) => {
    if (event.pointerType !== 'touch') revealTabs();
  });
  marker.addEventListener('click', () => {
    revealTabs();
    tabs.firstChild.focus({ preventScroll: true });
  });
  tabs.addEventListener('pointerenter', () => clearTimeout(leaveTimer));
  tabs.addEventListener('pointerleave', scheduleCollapse);
  preview.addEventListener('pointerleave', scheduleCollapse);
  tabs.addEventListener('focusout', scheduleCollapse);

  function updateControls() {
    const note = currentNote();
    if (!note) return;
    preview.style.setProperty('--note-color', colors[note.color].fill);
    preview.style.setProperty('--accent-color', colors[note.color].accent);
    sheet.style.setProperty('--note-font', note.font === 'virgil' ? 'Virgil' : 'Inter');
    sheet.setAttribute('aria-label', `Sample note: ${note.title.trim() || 'Untitled'}`);
    checklist.setAttribute('aria-pressed', String(note.lines[currentLine].checked !== null));
    pin.setAttribute('aria-pressed', String(alwaysOpen));
    $('pin-icon').src = `assets/icons/${alwaysOpen ? 'pin-filled' : 'pin'}.svg`;
    $('color-control').setAttribute('aria-label', `Note color: ${colors[note.color].name}. Change color`);
    $('color-panel').querySelectorAll('button').forEach((button) => {
      button.setAttribute('aria-pressed', String(button.dataset.color === note.color));
    });
    $('font-panel').querySelectorAll('button').forEach((button) => {
      button.setAttribute('aria-pressed', String(button.dataset.font === note.font));
    });
    updateTabs();
  }

  function fitTextareas() {
    if (active === null) return;
    body.querySelectorAll('textarea').forEach((input) => {
      input.style.height = 'auto';
      input.style.height = `${input.scrollHeight}px`;
    });
  }

  function focusLine(index, position = 0) {
    const input = body.querySelectorAll('textarea')[index];
    input.focus({ preventScroll: true });
    input.setSelectionRange(position, position);
    const fieldBounds = input.getBoundingClientRect();
    const bodyBounds = body.getBoundingClientRect();
    if (fieldBounds.bottom > bodyBounds.bottom) body.scrollTop += fieldBounds.bottom - bodyBounds.bottom;
    if (fieldBounds.top < bodyBounds.top) body.scrollTop -= bodyBounds.top - fieldBounds.top;
  }

  function renderLines(focusIndex = null, position = 0) {
    const note = currentNote();
    body.replaceChildren();
    note.lines.forEach((line, index) => {
      const row = document.createElement('div');
      row.className = `note-line${line.checked === true ? ' completed' : ''}`;
      let checkbox;
      if (line.checked !== null) {
        const hit = document.createElement('label');
        hit.className = 'check-hit';
        checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.className = 'line-checkbox';
        checkbox.checked = line.checked;
        checkbox.setAttribute('aria-label', `Complete: ${line.text || 'empty item'}`);
        checkbox.addEventListener('change', () => {
          currentLine = index;
          line.checked = checkbox.checked;
          row.classList.toggle('completed', line.checked);
          updateControls();
        });
        hit.append(checkbox);
        row.append(hit);
      }
      const input = document.createElement('textarea');
      input.rows = 1;
      input.value = line.text;
      input.spellcheck = false;
      input.autocomplete = 'off';
      input.setAttribute('aria-label', `Note text, line ${index + 1}`);
      const rememberCaret = () => {
        currentLine = index;
        caret = input.selectionStart;
        checklist.setAttribute('aria-pressed', String(line.checked !== null));
      };
      ['focus', 'select', 'click', 'keyup'].forEach((type) => input.addEventListener(type, rememberCaret));
      input.addEventListener('input', () => {
        line.text = input.value;
        rememberCaret();
        if (checkbox) checkbox.setAttribute('aria-label', `Complete: ${line.text || 'empty item'}`);
        fitTextareas();
      });
      input.addEventListener('keydown', (event) => {
        if (event.isComposing) return;
        const start = input.selectionStart;
        const end = input.selectionEnd;
        if (event.key === 'Enter' && !event.shiftKey) {
          event.preventDefault();
          if (!line.text && line.checked !== null) {
            line.checked = null;
            renderLines(index);
          } else {
            const next = { text: line.text.slice(end), checked: line.checked === null ? null : false };
            line.text = line.text.slice(0, start);
            note.lines.splice(index + 1, 0, next);
            renderLines(index + 1);
          }
        } else if (event.key === 'Backspace' && start === 0 && end === 0) {
          if (line.checked !== null) {
            event.preventDefault();
            line.checked = null;
            renderLines(index);
          } else if (index > 0) {
            event.preventDefault();
            const previous = note.lines[index - 1];
            const offset = previous.text.length;
            previous.text += line.text;
            note.lines.splice(index, 1);
            renderLines(index - 1, offset);
          }
        }
      });
      row.append(input);
      body.append(row);
    });
    currentLine = Math.min(currentLine, note.lines.length - 1);
    fitTextareas();
    if (focusIndex !== null) focusLine(focusIndex, position);
    updateControls();
  }

  function openNote(index) {
    const isSameNote = active === index;
    closePanel();
    if (!isSameNote) {
      active = index;
      alwaysOpen = false;
      currentLine = 0;
      caret = 0;
    }
    revealTabs();
    sheet.hidden = false;
    connector.hidden = false;
    // Three fixed Classic tabs: 104px high, separated by the native 8px gap.
    connector.style.top = `${46 + index * 112 + 17}px`;
    connector.classList.toggle('tab-hovered', tabs.children[index].matches(':hover'));
    title.value = notes[index].title;
    updateControls();
    renderLines();
    sheet.classList.remove('appearing');
    void sheet.offsetWidth;
    sheet.classList.add('appearing');
    $('close-note').focus({ preventScroll: true });
    armTimeout();
    announce(`${notes[index].title || 'Untitled'} opened. Edit its title or text, or try the note controls.`);
  }

  function closeNote() {
    if (active === null) return;
    const previous = active;
    closePanel();
    clearTimeout(idleTimer);
    active = null;
    alwaysOpen = false;
    sheet.hidden = true;
    connector.hidden = true;
    connector.classList.remove('tab-hovered');
    revealTabs();
    updateTabs();
    tabs.children[previous].focus({ preventScroll: true });
    announce('Note closed. Your demo edits are kept until refresh or Reset.');
  }

  function armTimeout() {
    clearTimeout(idleTimer);
    lastActivity = Date.now();
    if (active === null || alwaysOpen) return;
    const expire = () => {
      const remaining = idleMilliseconds - (Date.now() - lastActivity);
      if (remaining > 0) { idleTimer = setTimeout(expire, remaining); return; }
      // Do not move focus back into the demo after a visitor has left it.
      const hadFocus = preview.contains(document.activeElement);
      active = null;
      sheet.hidden = true;
      connector.hidden = true;
      closePanel();
      updateTabs();
      collapseTabs(hadFocus);
      announce('Note closed after one idle minute. Your demo edits are still here.');
    };
    idleTimer = setTimeout(expire, idleMilliseconds);
  }

  ['pointermove', 'pointerdown', 'keydown', 'input', 'focusin'].forEach((type) => {
    preview.addEventListener(type, () => { lastActivity = Date.now(); });
  });
  $('close-note').addEventListener('click', closeNote);
  title.addEventListener('input', () => {
    currentNote().title = title.value;
    updateTabs();
    sheet.setAttribute('aria-label', `Sample note: ${title.value.trim() || 'Untitled'}`);
  });

  function closePanel(restoreFocus = false) {
    if (!openPanel) return;
    const name = openPanel;
    $(`${name}-panel`).hidden = true;
    $(`${name}-control`).setAttribute('aria-expanded', 'false');
    openPanel = null;
    if (restoreFocus) $(`${name}-control`).focus({ preventScroll: true });
  }

  panels.forEach((name) => {
    $(`${name}-control`).addEventListener('click', () => {
      const wasOpen = openPanel === name;
      closePanel();
      if (wasOpen) return;
      openPanel = name;
      $(`${name}-panel`).hidden = false;
      $(`${name}-control`).setAttribute('aria-expanded', 'true');
      $(`${name}-panel`).querySelector('button').focus({ preventScroll: true });
    });
  });
  document.addEventListener('pointerdown', (event) => {
    if (openPanel && !event.target.closest(`#${openPanel}-panel, #${openPanel}-control`)) closePanel();
  });
  document.addEventListener('focusin', (event) => {
    if (openPanel && !event.target.closest(`#${openPanel}-panel, #${openPanel}-control`)) closePanel();
  });

  Object.entries(colors).forEach(([key, color]) => {
    const button = document.createElement('button');
    button.dataset.color = key;
    button.title = color.name;
    button.setAttribute('aria-label', color.name);
    const swatch = document.createElement('span');
    swatch.className = 'swatch';
    swatch.style.setProperty('--swatch', color.fill);
    button.append(swatch);
    button.addEventListener('click', () => {
      currentNote().color = key;
      updateControls();
      closePanel(true);
      announce(`Note color changed to ${color.name}.`);
    });
    $('color-panel').append(button);
  });
  $('font-panel').querySelectorAll('button').forEach((button) => {
    button.addEventListener('click', () => {
      currentNote().font = button.dataset.font;
      updateControls();
      fitTextareas();
      closePanel(true);
      announce(`Note font changed to ${button.textContent}.`);
    });
  });
  checklist.addEventListener('click', () => {
    closePanel();
    const line = currentNote().lines[currentLine];
    line.checked = line.checked === null ? false : null;
    renderLines(currentLine, caret);
    announce(line.checked === null ? 'Checklist removed from current line.' : 'Current line is now a checklist item.');
  });
  pin.addEventListener('click', () => {
    closePanel();
    alwaysOpen = !alwaysOpen;
    updateControls();
    armTimeout();
    announce(alwaysOpen ? 'Always open on. This note will stay open until you close it.' : 'Always open off. This note closes after one idle minute.');
  });
  $('copy-text').addEventListener('click', async () => {
    const text = currentNote().lines.map((line) =>
      `${line.checked === null ? '' : line.checked ? '- [x] ' : '- [ ] '}${line.text}`).join('\n');
    closePanel(true);
    try {
      await navigator.clipboard.writeText(text);
      announce('Note text copied.');
    } catch {
      announce('Clipboard unavailable. Select the note text and use your device’s Copy command.');
    }
  });

  function resetDemo() {
    clearTimeout(idleTimer);
    closePanel();
    notes = samples();
    active = null;
    currentLine = 0;
    caret = 0;
    alwaysOpen = false;
    sheet.hidden = true;
    connector.hidden = true;
    connector.classList.remove('tab-hovered');
    title.value = '';
    body.replaceChildren();
    updateTabs();
    collapseTabs(true);
    announce('Demo reset. Reveal the tabs to start again.');
  }
  $('reset-demo').addEventListener('click', resetDemo);
  $('reset-menu').addEventListener('click', resetDemo);
  preview.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape' || event.isComposing) return;
    if (openPanel) closePanel(true);
    else if (active !== null) closeNote();
    else if (preview.dataset.view === 'tabs') collapseTabs(true);
    else return;
    event.preventDefault();
    event.stopPropagation();
  });
  window.addEventListener('resize', fitTextareas);
  document.fonts.ready.then(fitTextareas);
  updateTabs();
  marker.disabled = false;
  $('reset-demo').disabled = false;
})();
