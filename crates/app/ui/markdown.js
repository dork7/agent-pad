function renderInline(parent, text) {
  const tokens = /`([^`]+)`|\[([^\]]+)\]\(((?:[^()\n]|\([^()\n]*\))*)\)|\*\*(.+?)\*\*|__(.+?)__/g;
  let end = 0;
  for (const match of text.matchAll(tokens)) {
    const [token, code, label, destination, bold, strong] = match;
    parent.append(text.slice(end, match.index));
    if (code !== undefined) parent.append(el('code', { textContent: code }));
    else if (label !== undefined) parent.append(`${label} (${destination})`);
    else parent.append(el('strong', { textContent: bold ?? strong }));
    end = match.index + token.length;
  }
  parent.append(text.slice(end));
}

// Markdown is untrusted. Only fixed element names and text nodes reach the DOM.
function renderMarkdown(markdown) {
  const article = el('article', { className: 'markdown', ariaLabel: 'Ticket Markdown' });
  const lists = [];
  let paragraph = null, fence = null;
  for (const line of markdown.split(/\r?\n/)) {
    const fenceMarker = line.match(/^\s*(`{3,}|~{3,})(.*)$/);
    if (fence) {
      if (fenceMarker && fenceMarker[1].startsWith(fence.marker) && !fenceMarker[2].trim()) fence = null;
      else fence.code.append(line + '\n');
      continue;
    }
    if (!line.trim()) {
      paragraph = null;
      continue;
    }
    const bullet = line.match(/^([ \t]*)([-+*]|\d+[.)])\s+(.*)$/);
    if (bullet) {
      const [, whitespace, marker, content] = bullet;
      const indent = whitespace.replace(/\t/g, '    ').length;
      const tag = /^\d/.test(marker) ? 'ol' : 'ul';
      while (lists.length && lists.at(-1).indent > indent) lists.pop();
      if (lists.at(-1)?.indent === indent && lists.at(-1).tag !== tag) lists.pop();
      if (!lists.length || lists.at(-1).indent < indent) {
        const list = tag === 'ol' ? el('ol', { start: parseInt(marker, 10) }) : el('ul');
        (lists.at(-1)?.list.lastElementChild || article).append(list);
        lists.push({ indent, tag, list });
      }
      const item = el('li');
      const task = content.match(/^\[([ xX])\]\s+(.*)$/);
      if (task) {
        const label = el('label', {}, el('input', { type: 'checkbox', disabled: true, checked: task[1] !== ' ' }));
        renderInline(label, task[2]);
        item.append(label);
      } else renderInline(item, content);
      lists.at(-1).list.append(item);
      paragraph = null;
      continue;
    }
    lists.length = 0;
    const heading = line.match(/^ {0,3}(#{1,6})[ \t]+(.+)$/);
    if (fenceMarker) {
      fence = { marker: fenceMarker[1], code: el('code') };
      article.append(el('pre', {}, fence.code));
      paragraph = null;
    } else if (heading) {
      const node = el(`h${Math.min(6, heading[1].length + 2)}`);
      renderInline(node, heading[2].replace(/[ \t]+#+[ \t]*$/, '').trim());
      article.append(node);
      paragraph = null;
    } else {
      if (!paragraph) {
        paragraph = el('p');
        article.append(paragraph);
      } else paragraph.append(' ');
      renderInline(paragraph, line.trim());
    }
  }
  return article;
}
