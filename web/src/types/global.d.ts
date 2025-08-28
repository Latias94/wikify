/**
 * 全局类型声明
 */

// 扩展 Window 接口以支持文件系统访问 API
declare global {
  interface Window {
    showDirectoryPicker?: () => Promise<FileSystemDirectoryHandle>;
  }

  // 扩展 HTMLInputElement 以支持 webkitdirectory 属性
  interface HTMLInputElement {
    webkitdirectory?: boolean;
  }

  // 扩展 File 接口以支持 webkitRelativePath
  interface File {
    webkitRelativePath: string;
  }

  // File System Access API 类型
  interface FileSystemHandle {
    kind: 'file' | 'directory';
    name: string;
  }

  interface FileSystemDirectoryHandle extends FileSystemHandle {
    kind: 'directory';
    entries(): AsyncIterableIterator<[string, FileSystemHandle]>;
    getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<FileSystemDirectoryHandle>;
    getFileHandle(name: string, options?: { create?: boolean }): Promise<FileSystemFileHandle>;
  }

  interface FileSystemFileHandle extends FileSystemHandle {
    kind: 'file';
    getFile(): Promise<File>;
  }
}

export {};
