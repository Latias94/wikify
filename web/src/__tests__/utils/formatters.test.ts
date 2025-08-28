/**
 * 格式化工具函数测试
 */

import { describe, it, expect } from 'vitest';
import {
  formatRelativeTime,
  formatDate,
  formatFileSize,
  formatNumber,
  formatPercentage,
  formatProgress,
  truncateText,
  formatFilePath,
  formatRepositoryName,
  formatGitUrl,
  detectLanguage,
  formatErrorMessage,
} from '@/utils/formatters';

describe('formatters', () => {
  describe('formatRelativeTime', () => {
    it('should format relative time correctly', () => {
      const now = new Date();
      const oneHourAgo = new Date(now.getTime() - 60 * 60 * 1000);
      
      const result = formatRelativeTime(oneHourAgo);
      expect(result).toContain('1 小时前');
    });

    it('should handle invalid dates', () => {
      const result = formatRelativeTime('invalid-date');
      expect(result).toBe('Invalid date');
    });
  });

  describe('formatDate', () => {
    it('should format date correctly', () => {
      const date = new Date('2024-01-15T10:30:00Z');
      const result = formatDate(date);
      expect(result).toBe('2024-01-15');
    });

    it('should handle custom format', () => {
      const date = new Date('2024-01-15T10:30:00Z');
      const result = formatDate(date, 'yyyy年MM月dd日');
      expect(result).toBe('2024年01月15日');
    });
  });

  describe('formatFileSize', () => {
    it('should format bytes correctly', () => {
      expect(formatFileSize(0)).toBe('0 B');
      expect(formatFileSize(1024)).toBe('1 KB');
      expect(formatFileSize(1024 * 1024)).toBe('1 MB');
      expect(formatFileSize(1024 * 1024 * 1024)).toBe('1 GB');
    });

    it('should handle decimal places', () => {
      expect(formatFileSize(1536)).toBe('1.5 KB');
      expect(formatFileSize(1024 * 1024 * 1.5)).toBe('1.5 MB');
    });
  });

  describe('formatNumber', () => {
    it('should format numbers with thousand separators', () => {
      expect(formatNumber(1000)).toBe('1,000');
      expect(formatNumber(1234567)).toBe('1,234,567');
    });
  });

  describe('formatPercentage', () => {
    it('should calculate percentage correctly', () => {
      expect(formatPercentage(50, 100)).toBe('50%');
      expect(formatPercentage(1, 3)).toBe('33%');
      expect(formatPercentage(0, 0)).toBe('0%');
    });
  });

  describe('formatProgress', () => {
    it('should clamp progress between 0 and 100', () => {
      expect(formatProgress(-10)).toBe('0%');
      expect(formatProgress(50)).toBe('50%');
      expect(formatProgress(150)).toBe('100%');
    });
  });

  describe('truncateText', () => {
    it('should truncate long text', () => {
      const longText = 'This is a very long text that should be truncated';
      const result = truncateText(longText, 20);
      expect(result).toBe('This is a very long...');
    });

    it('should not truncate short text', () => {
      const shortText = 'Short text';
      const result = truncateText(shortText, 20);
      expect(result).toBe('Short text');
    });
  });

  describe('formatFilePath', () => {
    it('should format long file paths', () => {
      const longPath = '/very/long/path/to/some/deeply/nested/file.txt';
      const result = formatFilePath(longPath, 20);
      expect(result).toBe('.../file.txt');
    });

    it('should keep short paths unchanged', () => {
      const shortPath = '/short/path.txt';
      const result = formatFilePath(shortPath, 50);
      expect(result).toBe('/short/path.txt');
    });
  });

  describe('formatRepositoryName', () => {
    it('should format repository names', () => {
      expect(formatRepositoryName('my-awesome-repo')).toBe('My Awesome Repo');
      expect(formatRepositoryName('snake_case_name')).toBe('Snake Case Name');
    });
  });

  describe('formatGitUrl', () => {
    it('should remove .git suffix', () => {
      const url = 'https://github.com/user/repo.git';
      const result = formatGitUrl(url);
      expect(result).toBe('https://github.com/user/repo');
    });

    it('should convert SSH to HTTPS', () => {
      const sshUrl = 'git@github.com:user/repo.git';
      const result = formatGitUrl(sshUrl);
      expect(result).toBe('https://github.com/user/repo');
    });
  });

  describe('detectLanguage', () => {
    it('should detect programming languages correctly', () => {
      expect(detectLanguage('file.js')).toBe('javascript');
      expect(detectLanguage('file.ts')).toBe('typescript');
      expect(detectLanguage('file.py')).toBe('python');
      expect(detectLanguage('file.rs')).toBe('rust');
      expect(detectLanguage('file.unknown')).toBe('text');
    });
  });

  describe('formatErrorMessage', () => {
    it('should format string errors', () => {
      const result = formatErrorMessage('Simple error message');
      expect(result).toBe('Simple error message');
    });

    it('should format Error objects', () => {
      const error = new Error('Error object message');
      const result = formatErrorMessage(error);
      expect(result).toBe('Error object message');
    });

    it('should format objects with message property', () => {
      const errorObj = { message: 'Object with message' };
      const result = formatErrorMessage(errorObj);
      expect(result).toBe('Object with message');
    });

    it('should format objects with error property', () => {
      const errorObj = { error: 'Object with error' };
      const result = formatErrorMessage(errorObj);
      expect(result).toBe('Object with error');
    });

    it('should handle unknown error types', () => {
      const result = formatErrorMessage(null);
      expect(result).toBe('An unknown error occurred');
    });
  });
});
