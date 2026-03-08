/**
 * 应用入口文件
 * 负责初始化 React 应用并将其挂载到 DOM 中
 */

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from './App';
import './index.css';

export const startApp = (): void => {
  const container = document.getElementById('root');

  if (!container) {
    throw new Error('未找到应用挂载节点 #root');
  }

  createRoot(container).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
};

startApp();
