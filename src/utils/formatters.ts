export function formatRelativeTime(unixTimestampSec: number): string {
  const nowSec = Math.floor(Date.now() / 1000);
  const diff = Math.max(0, nowSec - unixTimestampSec);

  if (diff < 30) return 'Just now';
  if (diff < 60) return `${diff}s ago`;

  const minutes = Math.floor(diff / 60);
  if (minutes < 60) return `${minutes}m ago`;

  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;

  const days = Math.floor(hours / 24);
  if (days === 1) return 'Yesterday';
  if (days < 7) return `${days}d ago`;

  return new Date(unixTimestampSec * 1000).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
  });
}

export function formatAppDisplayName(exeName: string): string {
  if (!exeName || exeName === 'Unknown') return 'App';
  
  const clean = exeName.replace(/\.exe$/i, '');
  const map: Record<string, string> = {
    Code: 'VS Code',
    chrome: 'Chrome',
    msedge: 'Edge',
    firefox: 'Firefox',
    WindowsTerminal: 'Terminal',
    powershell: 'PowerShell',
    cmd: 'Command Prompt',
    Discord: 'Discord',
    Slack: 'Slack',
    Notepad: 'Notepad',
    devenv: 'Visual Studio',
    idea64: 'IntelliJ',
    cursor: 'Cursor',
  };

  return map[clean] ?? clean;
}

export function tryFormatJson(raw: string): string | null {
  try {
    const parsed = JSON.parse(raw);
    return JSON.stringify(parsed, null, 2);
  } catch {
    return null;
  }
}

export function parseHexColor(hex: string): { rgb: string; hsl: string } | null {
  const cleanHex = hex.trim().replace(/^#/, '');
  if (![3, 4, 6, 8].includes(cleanHex.length)) return null;

  let r = 0, g = 0, b = 0;
  if (cleanHex.length === 3 || cleanHex.length === 4) {
    r = parseInt(cleanHex[0] + cleanHex[0], 16);
    g = parseInt(cleanHex[1] + cleanHex[1], 16);
    b = parseInt(cleanHex[2] + cleanHex[2], 16);
  } else {
    r = parseInt(cleanHex.substring(0, 2), 16);
    g = parseInt(cleanHex.substring(2, 4), 16);
    b = parseInt(cleanHex.substring(4, 6), 16);
  }

  if (isNaN(r) || isNaN(g) || isNaN(b)) return null;

  const rgb = `rgb(${r}, ${g}, ${b})`;

  // Convert to HSL
  const rf = r / 255, gf = g / 255, bf = b / 255;
  const max = Math.max(rf, gf, bf), min = Math.min(rf, gf, bf);
  let h = 0, s = 0;
  const l = (max + min) / 2;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case rf: h = (gf - bf) / d + (gf < bf ? 6 : 0); break;
      case gf: h = (bf - rf) / d + 2; break;
      case bf: h = (rf - gf) / d + 4; break;
    }
    h = Math.round(h * 60);
  }

  const hsl = `hsl(${h}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%)`;

  return { rgb, hsl };
}
