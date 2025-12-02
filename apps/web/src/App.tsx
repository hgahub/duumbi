import { ThemeProvider } from '@duumbi/ui-components';
import { useState } from 'react';
import ContentFrame from './components/ContentFrame';
import Sidebar from './components/Sidebar';
import TopMenu from './components/TopMenu';
import Preferences from './pages/Preferences';

function App() {
  const [isSidebarOpen, setIsSidebarOpen] = useState(false);
  const [currentView, setCurrentView] = useState<'home' | 'preferences'>(
    'home'
  );
  const [chatId, setChatId] = useState(0);

  const toggleSidebar = () => {
    setIsSidebarOpen(!isSidebarOpen);
  };

  const closeSidebar = () => {
    setIsSidebarOpen(false);
  };

  const handleNavigate = (view: 'home' | 'preferences') => {
    setCurrentView(view);
    if (window.innerWidth < 768) {
      closeSidebar();
    }
  };

  const handleNewChat = () => {
    setChatId((prev) => prev + 1);
    handleNavigate('home');
  };

  return (
    <ThemeProvider defaultTheme="system">
      <div className="flex h-screen w-full bg-gray-100 dark:bg-higashi-kashmirblue-900 text-gray-900 dark:text-gray-100">
        <Sidebar
          isOpen={isSidebarOpen}
          onClose={closeSidebar}
          onNavigate={handleNavigate}
          onNewChat={handleNewChat}
        />

        <div className="flex flex-col flex-1 w-full h-full">
          <TopMenu onMenuToggle={toggleSidebar} />
          {currentView === 'preferences' ? (
            <div className="flex-1 overflow-y-auto">
              <Preferences onBack={() => handleNavigate('home')} />
            </div>
          ) : (
            <ContentFrame key={chatId} />
          )}
        </div>
      </div>
    </ThemeProvider>
  );
}

export default App;
