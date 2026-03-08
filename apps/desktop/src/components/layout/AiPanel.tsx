import React from 'react';
import { Bot, X, Sparkles } from 'lucide-react';

interface AiPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

export const AiPanel: React.FC<AiPanelProps> = ({ isOpen, onClose }) => {
  if (!isOpen) return null;

  return (
    <aside className="w-[360px] bg-white border-l border-gray-200 flex flex-col shadow-sm transition-all duration-300 ease-in-out shrink-0">
      <div className="h-14 flex items-center justify-between px-4 border-b border-gray-100 bg-brand-50/50">
        <div className="flex items-center gap-2 text-brand-700 font-medium">
          <Bot className="w-5 h-5" />
          <span>AI 助手</span>
        </div>
        <button
          onClick={onClose}
          className="p-1.5 rounded-md text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-colors"
          aria-label="关闭 AI 助手"
        >
          <X className="w-4 h-4" />
        </button>
      </div>
      
      <div className="flex-1 overflow-y-auto p-4 flex flex-col items-center justify-center text-center text-gray-500">
        <div className="w-16 h-16 bg-brand-50 rounded-full flex items-center justify-center mb-4 text-brand-500">
          <Sparkles className="w-8 h-8" />
        </div>
        <h3 className="text-lg font-medium text-gray-900 mb-2">AI 助手就绪</h3>
        <p className="text-sm max-w-[200px]">
          我在这里协助您处理教务工作、分析成绩和生成评语。
        </p>
      </div>
      
      <div className="p-4 border-t border-gray-100 bg-gray-50">
        <div className="relative">
          <input
            type="text"
            placeholder="输入指令..."
            className="w-full pl-4 pr-10 py-2.5 bg-white border border-gray-300 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-brand-500 focus:border-transparent transition-shadow"
            disabled
          />
          <button 
            className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 text-gray-400 hover:text-brand-600 transition-colors"
            disabled
          >
            <Bot className="w-4 h-4" />
          </button>
        </div>
      </div>
    </aside>
  );
};
