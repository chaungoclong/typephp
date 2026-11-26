<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Http\Message;

require_once __DIR__ . '/MessageInterface.php';
require_once __DIR__ . '/ServerRequestInterface.php';
require_once __DIR__ . '/StreamInterface.php';
require_once __DIR__ . '/UriInterface.php';

class ServerRequest implements ServerRequestInterface
{
    private string $method;
    private UriInterface $uri;
    /** @var array<string, string> */
    private array $headers;
    private StreamInterface $body;
    /** @var array<string, mixed> */
    private array $queryParams;
    private mixed $parsedBody;
    /** @var array<string, mixed> */
    private array $attributes;

    /**
     * @param array<string, string> $headers
     * @param array<string, mixed> $queryParams
     * @param array<string, mixed> $attributes
     */
    public function __construct(string $method, UriInterface $uri, array $headers, StreamInterface $body, array $queryParams, mixed $parsedBody, array $attributes)
    {
        $this->method = strtoupper($method);
        $this->uri = $uri;
        $this->headers = $headers;
        $this->body = $body;
        $this->queryParams = $queryParams;
        $this->parsedBody = $parsedBody;
        $this->attributes = $attributes;
    }

    public function getProtocolVersion(): string
    {
        return '1.1';
    }

    public function getMethod(): string
    {
        return $this->method;
    }

    public function getUri(): UriInterface
    {
        return $this->uri;
    }

    /**
     * @return array<string, string>
     */
    public function getHeaders(): array
    {
        return $this->headers;
    }

    public function hasHeader(string $name): bool
    {
        return array_key_exists(strtolower($name), $this->headers);
    }

    public function getHeaderLine(string $name): string
    {
        $key = strtolower($name);
        if (array_key_exists($key, $this->headers)) {
            return $this->headers[$key];
        }
        return '';
    }

    public function withHeader(string $name, string $value): MessageInterface
    {
        /** @var array<string, string> $next */
        $next = $this->headers;
        $next[strtolower($name)] = $value;
        return new ServerRequest($this->method, $this->uri, $next, $this->body, $this->queryParams, $this->parsedBody, $this->attributes);
    }

    public function getBody(): StreamInterface
    {
        return $this->body;
    }

    public function withBody(StreamInterface $body): MessageInterface
    {
        return new ServerRequest($this->method, $this->uri, $this->headers, $body, $this->queryParams, $this->parsedBody, $this->attributes);
    }

    /**
     * @return array<string, mixed>
     */
    public function getQueryParams(): array
    {
        return $this->queryParams;
    }

    public function getParsedBody(): mixed
    {
        return $this->parsedBody;
    }

    public function getAttribute(string $name): mixed
    {
        if (array_key_exists($name, $this->attributes)) {
            return $this->attributes[$name];
        }
        return null;
    }

    public function withAttribute(string $name, mixed $value): ServerRequestInterface
    {
        /** @var array<string, mixed> $next */
        $next = $this->attributes;
        $next[$name] = $value;
        return new ServerRequest($this->method, $this->uri, $this->headers, $this->body, $this->queryParams, $this->parsedBody, $next);
    }
}
