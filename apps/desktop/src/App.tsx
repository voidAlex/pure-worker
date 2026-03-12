/**
 * 应用根组件
 * 配置 TanStack Query 客户端和 React Router 路由，注册所有页面路由。
 * 首次启动时显示初始化向导，引导用户完成工作目录和 AI 配置。
 */

import { useState, useEffect, type ReactElement } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter, Route, Routes } from 'react-router';
import { AppLayout } from '@/components/layout/AppLayout';
import { DashboardPage } from '@/pages/DashboardPage';
import { ClassesPage } from '@/pages/ClassesPage';
import { StudentsPage } from '@/pages/StudentsPage';
import { StudentDetailPage } from '@/pages/StudentDetailPage';
import { ImportPage } from '@/pages/ImportPage';
import { SchedulePage } from '@/pages/SchedulePage';
import { SettingsPage } from '@/pages/SettingsPage';
import { SemesterCommentsPage } from '@/pages/SemesterCommentsPage';
import { ActivityAnnouncementsPage } from '@/pages/ActivityAnnouncementsPage';
import { AssignmentGradingPage } from '@/pages/AssignmentGradingPage';
import { PracticeSheetsPage } from '@/pages/PracticeSheetsPage';
import { InitializationWizard } from '@/components/shared/InitializationWizard';
import { commands } from '@/services/commandClient';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

/** 主应用内容（路由及布局） */
const AppContent = (): ReactElement => (
  <BrowserRouter>
    <Routes>
      <Route element={<AppLayout />}>
        <Route path="/" element={<DashboardPage />} />
        <Route path="/classes" element={<ClassesPage />} />
        <Route path="/students" element={<StudentsPage />} />
        <Route path="/students/:id" element={<StudentDetailPage />} />
        <Route path="/import" element={<ImportPage />} />
        <Route path="/schedule" element={<SchedulePage />} />
        <Route path="/settings" element={<SettingsPage />} />
        <Route path="/semester-comments" element={<SemesterCommentsPage />} />
        <Route path="/announcements" element={<ActivityAnnouncementsPage />} />
        <Route path="/assignment-grading" element={<AssignmentGradingPage />} />
        <Route path="/practice-sheets" element={<PracticeSheetsPage />} />
      </Route>
    </Routes>
  </BrowserRouter>
);

export const App = (): ReactElement => {
  const [initialized, setInitialized] = useState<boolean | null>(null);

  useEffect(() => {
    commands
      .checkInitializationStatus()
      .then((res) => {
        if (res.status === 'ok') {
          setInitialized(res.data.initialized);
        } else {
          console.error('检查初始化状态失败，进入初始化向导:', res.error);
          setInitialized(false);
        }
      })
      .catch((err: unknown) => {
        console.error('检查初始化状态异常，进入初始化向导:', err);
        setInitialized(false);
      });
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      {initialized === null && (
        <div className="flex items-center justify-center h-screen bg-gray-50">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600" />
        </div>
      )}

      {initialized === false && (
        <InitializationWizard onComplete={() => setInitialized(true)} />
      )}

      {initialized === true && <AppContent />}
    </QueryClientProvider>
  );
};
