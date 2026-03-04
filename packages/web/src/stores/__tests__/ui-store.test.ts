import { describe, it, expect, beforeEach } from 'vitest';
import { useUiStore } from '../ui-store';

beforeEach(() => {
    useUiStore.setState({ currentPage: 'tasks' });
});

describe('useUiStore', () => {
    it('defaults to tasks page', () => {
        expect(useUiStore.getState().currentPage).toBe('tasks');
    });

    it('switches to agents page', () => {
        useUiStore.getState().setPage('agents');
        expect(useUiStore.getState().currentPage).toBe('agents');
    });

    it('switches back to tasks page', () => {
        useUiStore.getState().setPage('agents');
        useUiStore.getState().setPage('tasks');
        expect(useUiStore.getState().currentPage).toBe('tasks');
    });
});
