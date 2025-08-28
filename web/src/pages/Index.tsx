import RepositoryManager from "@/components/RepositoryManager";
import ThemeToggle from "@/components/ThemeToggle";

const Index = () => {
  return (
    <div className="min-h-screen">
      {/* Header with theme toggle */}
      <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-14 items-center justify-end">
          <ThemeToggle />
        </div>
      </header>

      {/* Main content */}
      <main className="py-8">
        <RepositoryManager />
      </main>
    </div>
  );
};

export default Index;
