import React, { useState } from 'react';
import { Outlet } from 'react-router';
import { Sidebar } from './Sidebar';
import { AiPanel } from './AiPanel';
import { StatusBar } from './StatusBar';
import { ToastContainer } from '../shared/Toast';
import { Bot } from 'lucide-react';

export const AppLayout: React.FC = () => {
  const [isSidebarCollapsed, setIsSidebarCollapsed] = useState(false);
  const [isAiPanelOpen, setIsAiPanelOpen] = useState(false);

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-gray-50 text-gray-900 font-sans">
      <div className="flex flex-1 overflow-hidden">
        <Sidebar 
          isCollapsed={isSidebarCollapsed} 
          toggleCollapse={() => setIsSidebarCollapsed(!isSidebarCollapsed)} 
        />
        
        <main className="flex-1 flex flex-col overflow-hidden relative">
          <div className="flex-1 overflow-y-auto p-6">
            <Outlet />
          </div>
          
          {!isAiPanelOpen && (
            <button
              onClick={() => setIsAiPanelOpen(true)}
              className="absolute bottom-6 right-6 p-3 bg-brand-600 text-white rounded-full shadow-lg hover:bg-brand-700 hover:shadow-xl transition-all duration-300 ease-in-out focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-brand-500 z-40"
              aria-label="打开 AI 助手"
            >
              <Bot className="w-6 h-6" />
            </button>
          )}
        </main>
        
        <AiPanel 
          isOpen={isAiPanelOpen} 
          onClose={() => setIsAiPanelOpen(false)} 
        />
      </div>
      
      <StatusBar />
      <ToastContainer />
    </div>
  );
};
