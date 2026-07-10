import { Link } from "react-router-dom";

export default function NotFound() {
  return (
    <div className="flex items-center justify-center min-h-screen">
      <div className="text-center space-y-6">
        <div className="space-y-2">
          <h1 className="text-6xl font-bold text-accent-primary">404</h1>
          <p className="page-title text-text-primary">Page not found</p>
        </div>
        <p className="text-text-secondary max-w-sm">
          The page you're looking for doesn't exist or has been moved.
        </p>
        <Link
          to="/"
          className="inline-flex items-center justify-center rounded-md bg-accent-primary text-white font-medium px-6 py-3 transition-colors duration-150 hover:bg-accent-hover"
        >
          Back to Dashboard
        </Link>
      </div>
    </div>
  );
}
