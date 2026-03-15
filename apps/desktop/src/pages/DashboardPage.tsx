/**
 * 工作台页面组件
 * 仪表盘首页，内嵌 AI 工作台，占满可用高度。
 */

import React, { useEffect, useState } from 'react';

import { ChatPanel } from '@/components/chat/ChatPanel';
import { commands, TeacherProfile } from '@/bindings';

/**
 * 仪表盘页面 —— 渲染顶部标题 + AI 工作台。
 */
export const DashboardPage: React.FC = () => {
  const [teacherProfile, setTeacherProfile] = useState<TeacherProfile | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadTeacherProfile = async () => {
      try {
        setIsLoading(true);
        const result = await commands.getTeacherProfile();
        if (result.status === 'ok') {
          setTeacherProfile(result.data);
          setError(null);
        } else {
          setError('获取教师档案失败');
        }
      } catch (e) {
        console.error('加载教师档案失败:', e);
        setError('加载教师档案失败');
      } finally {
        setIsLoading(false);
      }
    };

    loadTeacherProfile();
  }, []);

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-gray-500">加载中...</div>
      </div>
    );
  }

  if (error || !teacherProfile) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-red-500">{error || '无法获取教师档案'}</div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* 页面标题区 */}
      <header className="flex items-center justify-between shrink-0">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">工作台</h1>
          <p className="text-sm text-gray-500 mt-1">欢迎使用 PureWorker 教务助手</p>
        </div>
      </header>

      {/* AI 聊天面板，填满剩余空间 */}
      <div className="flex-1 min-h-0 mt-6">
        <ChatPanel agentRole="homeroom" teacherId={teacherProfile.id} />
      </div>
    </div>
  );
};
