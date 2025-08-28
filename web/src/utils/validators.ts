/**
 * 验证工具函数
 */

import { VALIDATION_RULES } from '@/lib/constants';

// ============================================================================
// 基础验证函数
// ============================================================================

/**
 * 验证是否为空
 */
export function isEmpty(value: unknown): boolean {
  if (value === null || value === undefined) return true;
  if (typeof value === 'string') return value.trim().length === 0;
  if (Array.isArray(value)) return value.length === 0;
  if (typeof value === 'object') return Object.keys(value).length === 0;
  return false;
}

/**
 * 验证字符串长度
 */
export function validateLength(
  value: string,
  min?: number,
  max?: number
): { isValid: boolean; error?: string } {
  const length = value.trim().length;
  
  if (min !== undefined && length < min) {
    return {
      isValid: false,
      error: `最少需要 ${min} 个字符`,
    };
  }
  
  if (max !== undefined && length > max) {
    return {
      isValid: false,
      error: `最多允许 ${max} 个字符`,
    };
  }
  
  return { isValid: true };
}

/**
 * 验证正则表达式
 */
export function validatePattern(
  value: string,
  pattern: RegExp,
  errorMessage: string = '格式不正确'
): { isValid: boolean; error?: string } {
  if (!pattern.test(value)) {
    return {
      isValid: false,
      error: errorMessage,
    };
  }
  
  return { isValid: true };
}

// ============================================================================
// URL 验证
// ============================================================================

/**
 * 验证 URL 格式
 */
export function validateUrl(url: string): { isValid: boolean; error?: string } {
  if (isEmpty(url)) {
    return {
      isValid: false,
      error: 'URL 不能为空',
    };
  }
  
  try {
    new URL(url);
    return { isValid: true };
  } catch {
    return {
      isValid: false,
      error: '请输入有效的 URL 地址',
    };
  }
}

/**
 * 验证 GitHub URL
 */
export function validateGitHubUrl(url: string): { isValid: boolean; error?: string } {
  const urlValidation = validateUrl(url);
  if (!urlValidation.isValid) return urlValidation;
  
  const githubPattern = /^https:\/\/github\.com\/[\w\-\.]+\/[\w\-\.]+\/?$/;
  const sshPattern = /^git@github\.com:[\w\-\.]+\/[\w\-\.]+\.git$/;
  
  if (!githubPattern.test(url) && !sshPattern.test(url)) {
    return {
      isValid: false,
      error: '请输入有效的 GitHub 仓库地址',
    };
  }
  
  return { isValid: true };
}

/**
 * 验证 Git URL
 */
export function validateGitUrl(url: string): { isValid: boolean; error?: string } {
  if (isEmpty(url)) {
    return {
      isValid: false,
      error: 'Git URL 不能为空',
    };
  }
  
  // 支持的 Git URL 格式
  const patterns = [
    /^https?:\/\/.+\.git$/,                    // HTTPS
    /^git@.+:.+\.git$/,                       // SSH
    /^ssh:\/\/git@.+\/.+\.git$/,              // SSH with protocol
    /^https?:\/\/github\.com\/.+\/.+\/?$/,    // GitHub HTTPS
    /^https?:\/\/gitlab\.com\/.+\/.+\/?$/,    // GitLab HTTPS
    /^https?:\/\/bitbucket\.org\/.+\/.+\/?$/, // Bitbucket HTTPS
  ];
  
  const isValid = patterns.some(pattern => pattern.test(url));
  
  if (!isValid) {
    return {
      isValid: false,
      error: '请输入有效的 Git 仓库地址',
    };
  }
  
  return { isValid: true };
}

// ============================================================================
// 文件路径验证
// ============================================================================

/**
 * 验证本地路径
 */
export function validateLocalPath(path: string): { isValid: boolean; error?: string } {
  if (isEmpty(path)) {
    return {
      isValid: false,
      error: '路径不能为空',
    };
  }
  
  // 基本路径格式验证
  const invalidChars = /[<>:"|?*]/;
  if (invalidChars.test(path)) {
    return {
      isValid: false,
      error: '路径包含无效字符',
    };
  }
  
  return { isValid: true };
}

/**
 * 验证文件扩展名
 */
export function validateFileExtension(
  filename: string,
  allowedExtensions: string[]
): { isValid: boolean; error?: string } {
  const ext = filename.split('.').pop()?.toLowerCase();
  
  if (!ext || !allowedExtensions.includes(ext)) {
    return {
      isValid: false,
      error: `只允许以下文件类型: ${allowedExtensions.join(', ')}`,
    };
  }
  
  return { isValid: true };
}

// ============================================================================
// 仓库验证
// ============================================================================

/**
 * 验证仓库名称
 */
export function validateRepositoryName(name: string): { isValid: boolean; error?: string } {
  if (isEmpty(name)) {
    return {
      isValid: false,
      error: '仓库名称不能为空',
    };
  }
  
  const lengthValidation = validateLength(
    name,
    VALIDATION_RULES.REPOSITORY.NAME.MIN_LENGTH,
    VALIDATION_RULES.REPOSITORY.NAME.MAX_LENGTH
  );
  
  if (!lengthValidation.isValid) return lengthValidation;
  
  const patternValidation = validatePattern(
    name,
    VALIDATION_RULES.REPOSITORY.NAME.PATTERN,
    '仓库名称只能包含字母、数字、连字符、下划线和空格'
  );
  
  if (!patternValidation.isValid) return patternValidation;
  
  return { isValid: true };
}

/**
 * 验证仓库描述
 */
export function validateRepositoryDescription(description: string): { isValid: boolean; error?: string } {
  if (isEmpty(description)) {
    return { isValid: true }; // 描述是可选的
  }
  
  return validateLength(
    description,
    undefined,
    VALIDATION_RULES.REPOSITORY.DESCRIPTION.MAX_LENGTH
  );
}

/**
 * 验证仓库路径
 */
export function validateRepositoryPath(
  path: string,
  type: 'local' | 'remote'
): { isValid: boolean; error?: string } {
  if (isEmpty(path)) {
    return {
      isValid: false,
      error: '仓库路径不能为空',
    };
  }
  
  const lengthValidation = validateLength(
    path,
    VALIDATION_RULES.REPOSITORY.PATH.MIN_LENGTH,
    VALIDATION_RULES.REPOSITORY.PATH.MAX_LENGTH
  );
  
  if (!lengthValidation.isValid) return lengthValidation;
  
  if (type === 'remote') {
    return validateGitUrl(path);
  } else {
    return validateLocalPath(path);
  }
}

// ============================================================================
// 聊天验证
// ============================================================================

/**
 * 验证聊天消息
 */
export function validateChatMessage(message: string): { isValid: boolean; error?: string } {
  if (isEmpty(message)) {
    return {
      isValid: false,
      error: '消息不能为空',
    };
  }
  
  return validateLength(
    message,
    VALIDATION_RULES.CHAT.MESSAGE.MIN_LENGTH,
    VALIDATION_RULES.CHAT.MESSAGE.MAX_LENGTH
  );
}

// ============================================================================
// 会话验证
// ============================================================================

/**
 * 验证会话名称
 */
export function validateSessionName(name: string): { isValid: boolean; error?: string } {
  if (isEmpty(name)) {
    return {
      isValid: false,
      error: '会话名称不能为空',
    };
  }
  
  return validateLength(
    name,
    VALIDATION_RULES.SESSION.NAME.MIN_LENGTH,
    VALIDATION_RULES.SESSION.NAME.MAX_LENGTH
  );
}

// ============================================================================
// 表单验证
// ============================================================================

/**
 * 验证表单字段
 */
export function validateField(
  value: unknown,
  rules: {
    required?: boolean;
    minLength?: number;
    maxLength?: number;
    pattern?: RegExp;
    custom?: (value: unknown) => { isValid: boolean; error?: string };
  }
): { isValid: boolean; error?: string } {
  // 必填验证
  if (rules.required && isEmpty(value)) {
    return {
      isValid: false,
      error: '此字段为必填项',
    };
  }
  
  // 如果值为空且不是必填，则通过验证
  if (isEmpty(value) && !rules.required) {
    return { isValid: true };
  }
  
  const stringValue = String(value);
  
  // 长度验证
  if (rules.minLength !== undefined || rules.maxLength !== undefined) {
    const lengthValidation = validateLength(stringValue, rules.minLength, rules.maxLength);
    if (!lengthValidation.isValid) return lengthValidation;
  }
  
  // 正则验证
  if (rules.pattern) {
    const patternValidation = validatePattern(stringValue, rules.pattern);
    if (!patternValidation.isValid) return patternValidation;
  }
  
  // 自定义验证
  if (rules.custom) {
    const customValidation = rules.custom(value);
    if (!customValidation.isValid) return customValidation;
  }
  
  return { isValid: true };
}

/**
 * 验证整个表单
 */
export function validateForm<T extends Record<string, unknown>>(
  data: T,
  rules: Record<keyof T, Parameters<typeof validateField>[1]>
): { isValid: boolean; errors: Partial<Record<keyof T, string>> } {
  const errors: Partial<Record<keyof T, string>> = {};
  let isValid = true;
  
  for (const [field, fieldRules] of Object.entries(rules)) {
    const validation = validateField(data[field as keyof T], fieldRules);
    
    if (!validation.isValid) {
      errors[field as keyof T] = validation.error;
      isValid = false;
    }
  }
  
  return { isValid, errors };
}

// ============================================================================
// 工具函数
// ============================================================================

/**
 * 清理和标准化输入
 */
export function sanitizeInput(input: string): string {
  return input.trim().replace(/\s+/g, ' ');
}

/**
 * 检查是否为有效的 UUID
 */
export function isValidUUID(uuid: string): boolean {
  const uuidPattern = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
  return uuidPattern.test(uuid);
}

/**
 * 检查是否为有效的邮箱
 */
export function isValidEmail(email: string): boolean {
  const emailPattern = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailPattern.test(email);
}

/**
 * 检查密码强度
 */
export function validatePasswordStrength(password: string): {
  isValid: boolean;
  score: number;
  feedback: string[];
} {
  const feedback: string[] = [];
  let score = 0;
  
  if (password.length >= 8) {
    score += 1;
  } else {
    feedback.push('密码至少需要8个字符');
  }
  
  if (/[a-z]/.test(password)) {
    score += 1;
  } else {
    feedback.push('密码需要包含小写字母');
  }
  
  if (/[A-Z]/.test(password)) {
    score += 1;
  } else {
    feedback.push('密码需要包含大写字母');
  }
  
  if (/\d/.test(password)) {
    score += 1;
  } else {
    feedback.push('密码需要包含数字');
  }
  
  if (/[^a-zA-Z\d]/.test(password)) {
    score += 1;
  } else {
    feedback.push('密码需要包含特殊字符');
  }
  
  return {
    isValid: score >= 3,
    score,
    feedback,
  };
}
