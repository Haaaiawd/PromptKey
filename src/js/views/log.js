import { $, esc, fmtClock } from '../dom.js';
import { t } from '../i18n.js';
import { toast, confirmModal } from '../toast.js';
import { ipc } from '../store.js';

export async function renderLog() {
  const body = $('#logBody');
  if (!body) return;
  let logs;
  try {
    logs = await ipc('get_usage_logs', {}, { silent: true });
  } catch (e) {
    body.innerHTML = '';
    $('#logEmpty')?.classList.remove('hidden');
    return;
  }
  const total = logs.length;
  const okN = logs.filter(l => l.success).length;
  const times = logs.filter(l => l.injection_time_ms > 0).map(l => l.injection_time_ms);
  const stratCount = {};
  logs.forEach(l => { stratCount[l.strategy] = (stratCount[l.strategy] || 0) + 1; });
  const topStrat = Object.entries(stratCount).sort((a, b) => b[1] - a[1])[0]?.[0];

  $('#stTotal').textContent = String(total);
  $('#stRate').textContent = total ? `${Math.round(okN / total * 1000) / 10}%` : '–';
  $('#stRate').style.color = total && okN / total < 0.95 ? 'var(--danger)' : 'var(--success)';
  $('#stAvg').textContent = times.length ? `${Math.round(times.reduce((a, b) => a + b, 0) / times.length)}ms` : '–';
  $('#stStrategy').textContent = topStrat || '–';

  $('#logEmpty')?.classList.toggle('hidden', total > 0);
  body.innerHTML = logs.map(r => `<tr>
    <td>${esc(fmtClock(r.created_at))}</td>
    <td>${esc(r.prompt_name)}</td>
    <td title="${esc(r.window_title || '')}">${esc(r.target_app || '—')}</td>
    <td>${esc(r.strategy || '—')}</td>
    <td>${r.injection_time_ms > 0 ? esc(String(r.injection_time_ms) + 'ms') : '—'}</td>
    <td><span class="badge ${r.success ? 'ok' : 'err'}" title="${esc(r.error || '')}">${esc(t(r.success ? 'log.ok' : 'log.fail'))}</span></td>
  </tr>`).join('');
}

export function wireLog() {
  $('#logRefresh')?.addEventListener('click', renderLog);
  $('#logClear')?.addEventListener('click', async () => {
    if (!await confirmModal({ title: t('log.clear'), body: t('log.clearAsk'), okText: t('log.clear') })) return;
    try { await ipc('clear_usage_logs'); await renderLog(); toast('ok', t('log.cleared')); }
    catch { /* toasted */ }
  });
}
