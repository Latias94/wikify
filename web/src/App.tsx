import { Toaster } from "@/components/ui/toaster";
import { Toaster as Sonner } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import Index from "./pages/Index";
import NotFound from "./pages/NotFound";
import LoginPage from "./pages/Login";
import RegisterPage from "./pages/Register";
import ResearchPage from "./pages/Research";
import StreamingDemoPage from "./pages/StreamingDemo";
import { ChatInterface } from "./components/ChatInterface";
import { WikiViewer } from "./components/WikiViewer";
import { AuthProvider, AuthModeDetector, OpenSourceBadge } from "./components/AuthProvider";


const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1, // Only retry once on failure
      refetchOnWindowFocus: false, // Don't refetch on window focus
      refetchOnReconnect: false, // Don't refetch on reconnect
    },
    mutations: {
      retry: false, // Disable mutation retry to prevent duplicate initialization
    },
  },
});

const App = () => (
  <QueryClientProvider client={queryClient}>
    <TooltipProvider>
      <Toaster />
      <Sonner />
      <BrowserRouter>
        <AuthProvider>
          <AuthModeDetector>
            <Routes>
              <Route path="/" element={<Index />} />
              <Route path="/login" element={<LoginPage />} />
              <Route path="/register" element={<RegisterPage />} />
              <Route path="/chat/:repositoryId" element={<ChatInterface />} />
              <Route path="/wiki/:repositoryId" element={<WikiViewer />} />
              <Route path="/research/:repositoryId" element={<ResearchPage />} />
              <Route path="/streaming-demo" element={<StreamingDemoPage />} />

              {/* ADD ALL CUSTOM ROUTES ABOVE THE CATCH-ALL "*" ROUTE */}
              <Route path="*" element={<NotFound />} />
            </Routes>
            <OpenSourceBadge />
          </AuthModeDetector>
        </AuthProvider>
      </BrowserRouter>
    </TooltipProvider>
  </QueryClientProvider>
);

export default App;
