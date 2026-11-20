// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, cleanup, within } from '@testing-library/react';
import { ApplicationCatalog, matchesStatusFilter } from '../ApplicationCatalog';
import { getAppInitials } from '../ApplicationCard';
import type { ApplicationDto } from '../../lib/ipc';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

const make = (
  n: number,
  name: string,
  availability: ApplicationDto['availability'],
  sources: string[] = ['Start Menu (User)']
): ApplicationDto => ({
  id: `app_${String(n).padStart(32, '0')}`,
  displayName: name,
  publisher: 'Contoso',
  version: '1.0.0',
  sources,
  availability,
  isInstalled: true,
});

const apps: ApplicationDto[] = [
  make(1, 'Alpha Editor', 'Available'),
  make(2, 'Beta Browser', 'Available', ['Registry (HKLM 64-bit)']),
  make(3, 'Gamma Tool', 'Unverified'),
  make(4, 'Delta Legacy', 'MissingTarget'),
  make(5, 'Epsilon Kit', 'IncompleteMetadata'),
];

const rows = () => within(screen.getByRole('listbox', { name: 'Apps' })).getAllByRole('option');

describe('Apps view (Phase 2)', () => {
  beforeEach(() => vi.clearAllMocks());
  afterEach(() => cleanup());

  it('getAppInitials builds a two-letter monogram', () => {
    expect(getAppInitials('Visual Studio Code')).toBe('VS');
    expect(getAppInitials('7-Zip')).toBe('7Z');
    expect(getAppInitials('notepad')).toBe('N');
    expect(getAppInitials('  ')).toBe('?');
  });

  it('matchesStatusFilter splits Ready from everything that needs attention', () => {
    expect(matchesStatusFilter(apps[0], 'ready')).toBe(true);
    expect(matchesStatusFilter(apps[2], 'ready')).toBe(false);
    expect(matchesStatusFilter(apps[2], 'attention')).toBe(true);
    expect(matchesStatusFilter(apps[3], 'attention')).toBe(true);
    expect(matchesStatusFilter(apps[0], 'all')).toBe(true);
  });

  it('renders apps as a listbox with a single tab stop on the selected row', () => {
    render(<ApplicationCatalog initialApplications={apps} autoDiscover={false} />);

    const list = rows();
    expect(list).toHaveLength(5);
    expect(list[0].getAttribute('aria-selected')).toBe('true');
    expect(list.filter((r) => r.tabIndex === 0)).toHaveLength(1);
    expect(list[0].tabIndex).toBe(0);

    // Moving the selection moves the tab stop with it.
    fireEvent.keyDown(list[0], { key: 'ArrowDown' });
    const after = rows();
    expect(after[1].tabIndex).toBe(0);
    expect(after[0].tabIndex).toBe(-1);
    expect(document.activeElement).toBe(after[1]);
  });

  it('filters by status chips and shows counts on each chip', () => {
    render(<ApplicationCatalog initialApplications={apps} autoDiscover={false} />);

    const group = screen.getByRole('group', { name: 'Filter by status' });
    const ready = within(group).getByRole('button', { name: /^Ready/ });
    const attention = within(group).getByRole('button', { name: /^Needs attention/ });
    expect(ready.textContent).toContain('2');
    expect(attention.textContent).toContain('3');

    fireEvent.click(ready);
    expect(ready.getAttribute('aria-pressed')).toBe('true');
    expect(rows().map((r) => r.textContent)).toEqual([
      expect.stringContaining('Alpha Editor'),
      expect.stringContaining('Beta Browser'),
    ]);
    expect(screen.getByText('Showing 2 of 5 apps')).toBeDefined();

    fireEvent.click(attention);
    expect(rows()).toHaveLength(3);
  });

  it('combines the source dropdown with status chips', () => {
    render(<ApplicationCatalog initialApplications={apps} autoDiscover={false} />);

    fireEvent.change(screen.getByRole('combobox', { name: /where the app was found/i }), {
      target: { value: 'hklm-64' },
    });
    expect(rows()).toHaveLength(1);
    expect(rows()[0].textContent).toContain('Beta Browser');

    // Chip counts follow the source filter.
    const group = screen.getByRole('group', { name: 'Filter by status' });
    expect(within(group).getByRole('button', { name: /^All/ }).textContent).toContain('1');
  });

  it('keeps provenance collapsed and puts Open before the metadata', () => {
    render(<ApplicationCatalog initialApplications={apps} autoDiscover={false} />);

    const inspector = screen.getByTestId('application-inspector');
    const details = inspector.querySelector('details.app-inspector-provenance') as HTMLDetailsElement;
    expect(details).not.toBeNull();
    expect(details.open).toBe(false);
    expect(details.textContent).toContain('Start Menu (User)');
    expect(details.textContent).toContain('ID: app_');

    const open = within(inspector).getByRole('button', { name: 'Open Alpha Editor' });
    const ledger = within(inspector).getByRole('region', { name: 'Metadata Details' });
    // DOCUMENT_POSITION_FOLLOWING (4): the ledger comes after the Open button.
    expect(open.compareDocumentPosition(ledger) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });
});
