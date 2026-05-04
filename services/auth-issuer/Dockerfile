FROM golang:1.26-alpine AS build
WORKDIR /src
COPY go.mod go.sum ./
RUN go mod download
COPY . ./
RUN CGO_ENABLED=0 go build -o /out/auth-issuer .

FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /out/auth-issuer /usr/local/bin/auth-issuer
EXPOSE 8081
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/auth-issuer"]
