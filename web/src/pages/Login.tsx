/**
 * 登录页面
 */

import React from 'react';
import { Navigate, useSearchParams } from 'react-router-dom';
import { LoginForm } from '@/components/auth/LoginForm';
import { AuthModeConditional, useAuthContext } from '@/components/AuthProvider';
import { useAuth } from '@/hooks/use-auth';
import { Folder } from 'lucide-react';

const LoginPage: React.FC = () => {
  const [searchParams] = useSearchParams();
  const { authStatus } = useAuthContext();
  const { isAuthenticated } = useAuth();
  
  const redirectTo = searchParams.get('redirect') || '/';

  // 如果已经登录，重定向到目标页面
  if (isAuthenticated) {
    return <Navigate to={redirectTo} replace />;
  }

  // 如果是开放模式，不需要登录
  if (authStatus?.auth_mode === 'open') {
    return <Navigate to={redirectTo} replace />;
  }

  return (
    <AuthModeConditional
      modes={['private', 'enterprise']}
      fallback={<Navigate to="/" replace />}
    >
      <div className="min-h-screen flex items-center justify-center bg-background px-4">
        <div className="w-full max-w-md space-y-8">
          {/* Header */}
          <div className="text-center space-y-4">
            <div className="flex items-center justify-center gap-3">
              <Folder className="h-8 w-8 text-primary" />
              <h1 className="text-2xl font-bold text-foreground">Wikify</h1>
            </div>
            <p className="text-muted-foreground">
              Sign in to access your AI-powered code documentation
            </p>
          </div>

          {/* Login Form */}
          <LoginForm redirectTo={redirectTo} />

          {/* Footer */}
          <div className="text-center text-xs text-muted-foreground">
            <p>
              By signing in, you agree to our terms of service and privacy policy.
            </p>
          </div>
        </div>
      </div>
    </AuthModeConditional>
  );
};

export default LoginPage;
