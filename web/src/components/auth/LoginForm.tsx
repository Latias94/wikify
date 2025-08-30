/**
 * 登录表单组件
 * 提供用户登录功能
 */

import React, { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { useAuth } from '@/hooks/use-auth';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, Eye, EyeOff, LogIn, AlertCircle } from 'lucide-react';

// ============================================================================
// 表单验证 Schema
// ============================================================================

const loginSchema = z.object({
  username: z.string().min(1, 'Username is required'),
  password: z.string().min(1, 'Password is required'),
});

type LoginFormData = z.infer<typeof loginSchema>;

// ============================================================================
// 组件
// ============================================================================

interface LoginFormProps {
  onSuccess?: () => void;
  redirectTo?: string;
}

export const LoginForm: React.FC<LoginFormProps> = ({
  onSuccess,
  redirectTo = '/',
}) => {
  const navigate = useNavigate();
  const { login, isLoggingIn } = useAuth();
  const [showPassword, setShowPassword] = useState(false);
  const [loginError, setLoginError] = useState<string | null>(null);

  const {
    register,
    handleSubmit,
    formState: { errors, isValid },
  } = useForm<LoginFormData>({
    resolver: zodResolver(loginSchema),
    mode: 'onChange',
  });

  const onSubmit = async (data: LoginFormData) => {
    try {
      setLoginError(null);
      
      await new Promise<void>((resolve, reject) => {
        login(data, {
          onSuccess: () => {
            resolve();
            if (onSuccess) {
              onSuccess();
            } else {
              navigate(redirectTo);
            }
          },
          onError: (error: any) => {
            setLoginError(error.message || 'Login failed');
            reject(error);
          },
        });
      });
    } catch (error) {
      // Error is already handled in the onError callback
      console.error('Login error:', error);
    }
  };

  return (
    <Card className="w-full max-w-md mx-auto">
      <CardHeader className="space-y-1">
        <CardTitle className="text-2xl font-bold text-center">
          Sign In
        </CardTitle>
        <CardDescription className="text-center">
          Enter your credentials to access Wikify
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          {/* Username Field */}
          <div className="space-y-2">
            <Label htmlFor="username">Username</Label>
            <Input
              id="username"
              type="text"
              placeholder="Enter your username"
              autoComplete="username"
              disabled={isLoggingIn}
              {...register('username')}
              className={errors.username ? 'border-destructive' : ''}
            />
            {errors.username && (
              <p className="text-sm text-destructive">
                {errors.username.message}
              </p>
            )}
          </div>

          {/* Password Field */}
          <div className="space-y-2">
            <Label htmlFor="password">Password</Label>
            <div className="relative">
              <Input
                id="password"
                type={showPassword ? 'text' : 'password'}
                placeholder="Enter your password"
                autoComplete="current-password"
                disabled={isLoggingIn}
                {...register('password')}
                className={errors.password ? 'border-destructive pr-10' : 'pr-10'}
              />
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                onClick={() => setShowPassword(!showPassword)}
                disabled={isLoggingIn}
              >
                {showPassword ? (
                  <EyeOff className="h-4 w-4" />
                ) : (
                  <Eye className="h-4 w-4" />
                )}
                <span className="sr-only">
                  {showPassword ? 'Hide password' : 'Show password'}
                </span>
              </Button>
            </div>
            {errors.password && (
              <p className="text-sm text-destructive">
                {errors.password.message}
              </p>
            )}
          </div>

          {/* Error Alert */}
          {loginError && (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>{loginError}</AlertDescription>
            </Alert>
          )}

          {/* Submit Button */}
          <Button
            type="submit"
            className="w-full"
            disabled={isLoggingIn || !isValid}
          >
            {isLoggingIn ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Signing In...
              </>
            ) : (
              <>
                <LogIn className="mr-2 h-4 w-4" />
                Sign In
              </>
            )}
          </Button>

          {/* Register Link */}
          <div className="text-center text-sm">
            <span className="text-muted-foreground">Don't have an account? </span>
            <Link
              to="/register"
              className="text-primary hover:underline font-medium"
            >
              Sign up
            </Link>
          </div>
        </form>
      </CardContent>
    </Card>
  );
};

// ============================================================================
// 简化版登录表单（用于模态框等）
// ============================================================================

interface CompactLoginFormProps {
  onSuccess?: () => void;
  onCancel?: () => void;
}

export const CompactLoginForm: React.FC<CompactLoginFormProps> = ({
  onSuccess,
  onCancel,
}) => {
  const { login, isLoggingIn } = useAuth();
  const [showPassword, setShowPassword] = useState(false);
  const [loginError, setLoginError] = useState<string | null>(null);

  const {
    register,
    handleSubmit,
    formState: { errors, isValid },
  } = useForm<LoginFormData>({
    resolver: zodResolver(loginSchema),
    mode: 'onChange',
  });

  const onSubmit = async (data: LoginFormData) => {
    try {
      setLoginError(null);
      
      await new Promise<void>((resolve, reject) => {
        login(data, {
          onSuccess: () => {
            resolve();
            onSuccess?.();
          },
          onError: (error: any) => {
            setLoginError(error.message || 'Login failed');
            reject(error);
          },
        });
      });
    } catch (error) {
      console.error('Login error:', error);
    }
  };

  return (
    <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
      {/* Username Field */}
      <div className="space-y-2">
        <Label htmlFor="compact-username">Username</Label>
        <Input
          id="compact-username"
          type="text"
          placeholder="Username"
          autoComplete="username"
          disabled={isLoggingIn}
          {...register('username')}
          className={errors.username ? 'border-destructive' : ''}
        />
        {errors.username && (
          <p className="text-sm text-destructive">
            {errors.username.message}
          </p>
        )}
      </div>

      {/* Password Field */}
      <div className="space-y-2">
        <Label htmlFor="compact-password">Password</Label>
        <div className="relative">
          <Input
            id="compact-password"
            type={showPassword ? 'text' : 'password'}
            placeholder="Password"
            autoComplete="current-password"
            disabled={isLoggingIn}
            {...register('password')}
            className={errors.password ? 'border-destructive pr-10' : 'pr-10'}
          />
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
            onClick={() => setShowPassword(!showPassword)}
            disabled={isLoggingIn}
          >
            {showPassword ? (
              <EyeOff className="h-4 w-4" />
            ) : (
              <Eye className="h-4 w-4" />
            )}
          </Button>
        </div>
        {errors.password && (
          <p className="text-sm text-destructive">
            {errors.password.message}
          </p>
        )}
      </div>

      {/* Error Alert */}
      {loginError && (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{loginError}</AlertDescription>
        </Alert>
      )}

      {/* Action Buttons */}
      <div className="flex gap-2">
        {onCancel && (
          <Button
            type="button"
            variant="outline"
            className="flex-1"
            onClick={onCancel}
            disabled={isLoggingIn}
          >
            Cancel
          </Button>
        )}
        <Button
          type="submit"
          className="flex-1"
          disabled={isLoggingIn || !isValid}
        >
          {isLoggingIn ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Signing In...
            </>
          ) : (
            'Sign In'
          )}
        </Button>
      </div>
    </form>
  );
};

export default LoginForm;
