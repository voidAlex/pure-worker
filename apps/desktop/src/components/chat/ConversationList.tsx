import React from 'react';
import { ConversationListItem } from '@/services/chatService';

export interface ConversationListProps {
  conversations: ConversationListItem[];
  currentId?: string;
  onSelect: (id: string) => void;
  onCreateNew: () => void;
}

export function ConversationList({
  conversations,
  currentId,
  onSelect,
  onCreateNew,
}: ConversationListProps) {
  return (
    <div className="w-64 bg-gray-50 border-r border-gray-200 flex flex-col h-full">
      {/* 头部 */}
      <div className="p-4 border-b border-gray-200">
        <button
          onClick={onCreateNew}
          className="w-full py-2 px-4 bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors flex items-center justify-center gap-2"
        >
          <span>+</span>
          <span>新会话</span>
        </button>
      </div>

      {/* 会话列表 */}
      <div className="flex-1 overflow-y-auto">
        {conversations.length === 0 ? (
          <div className="p-4 text-center text-gray-400 text-sm">暂无会话记录</div>
        ) : (
          <div className="divide-y divide-gray-100">
            {conversations.map((conv) => (
              <button
                key={conv.id}
                onClick={() => onSelect(conv.id)}
                className={`w-full p-3 text-left hover:bg-gray-100 transition-colors ${
                  currentId === conv.id ? 'bg-blue-50 border-l-4 border-blue-500' : ''
                }`}
              >
                <div className="font-medium text-sm text-gray-800 truncate">
                  {conv.title || '未命名会话'}
                </div>
                <div className="text-xs text-gray-500 mt-1">
                  {conv.message_count} 条消息 · {new Date(conv.updated_at).toLocaleDateString()}
                </div>
                {conv.scenario && <div className="text-xs text-blue-500 mt-1">{conv.scenario}</div>}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
