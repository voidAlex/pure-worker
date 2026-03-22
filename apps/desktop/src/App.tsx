/**
 * 应用根组件
 * 配置 TanStack Query 客户端和 React Router 路由，注册所有页面路由。
 * 首次启动时显示初始化向导，引导用户完成工作目录和 AI 配置。
 */

import {
  useState,
  useEffect,
  type ReactElement,
  Component,
  type ErrorInfo,
  type ReactNode,
} from 'react';
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
import { LessonRecordsPage } from '@/pages/LessonRecordsPage';
import { InitializationWizard } from '@/components/shared/InitializationWizard';
import { commands } from '@/services/commandClient';

/** 初始化检查超时时间（毫秒） */
const INIT_TIMEOUT_MS = 10000;

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

/** 错误边界组件 - 捕获渲染错误并显示友好提示 */
class ErrorBoundary extends Component<
  { children: ReactNode },
  { hasError: boolean; error: Error | null }
> {
  constructor(props: { children: ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('应用渲染错误:', error);
    console.error('错误堆栈:', errorInfo.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center h-screen bg-gray-50 p-8">
          <div className="text-red-600 text-xl mb-4">应用加载出错</div>
          <div className="text-gray-600 text-sm mb-4">{this.state.error?.message}</div>
          <button
            onClick={() => window.location.reload()}
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
          >
            重新加载
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

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
        <Route path="/lesson-records" element={<LessonRecordsPage />} />
      </Route>
    </Routes>
  </BrowserRouter>
);

export const App = (): ReactElement => {
  const [initialized, setInitialized] = useState<boolean | null>(null);
  const [initError, setInitError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    let timeoutId: ReturnType<typeof setTimeout>;

    const checkInit = async () => {
      try {
        // 设置超时
        const timeoutPromise = new Promise<never>((_, reject) => {
          timeoutId = setTimeout(() => {
            reject(new Error('初始化检查超时，请检查应用是否正常运行'));
          }, INIT_TIMEOUT_MS);
        });

        const result = await Promise.race([commands.checkInitializationStatus(), timeoutPromise]);

        if (!mounted) return;

        clearTimeout(timeoutId);

        if (result.status === 'ok') {
          console.log('[App] 初始化状态检查完成:', result.data);
          setInitialized(result.data.initialized);
        } else {
          console.error('[App] 检查初始化状态失败，进入初始化向导:', result.error);
          setInitialized(false);
        }
      } catch (err) {
        if (!mounted) return;
        clearTimeout(timeoutId);
        console.error('[App] 检查初始化状态异常:', err);
        const errorMessage = err instanceof Error ? err.message : String(err);
        setInitError(errorMessage);
        // 超时或异常时也进入初始化向导
        setInitialized(false);
      }
    };

    checkInit();

    return () => {
      mounted = false;
      clearTimeout(timeoutId);
    };
  }, []);

  // 显示错误信息（如果有）
  if (initError) {
    console.warn('[App] 初始化遇到问题，但将继续启动:', initError);
  }

  return (
    <QueryClientProvider client={queryClient}>
      <ErrorBoundary>
        {initialized === null && (
          <div className="flex flex-col items-center justify-center h-screen bg-gray-50">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-brand-600 mb-4" />
            <div className="text-gray-500 text-sm">正在初始化应用...</div>
          </div>
        )}

        {initialized === false && <InitializationWizard onComplete={() => setInitialized(true)} />}

        {initialized === true && <AppContent />}
      </ErrorBoundary>
    </QueryClientProvider>
  );
};
