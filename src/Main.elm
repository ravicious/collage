port module Main exposing (main)

import Browser
import File exposing (File)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (..)
import Json.Decode as D


port sendImagesToJs : List D.Value -> Cmd msg


port imageProcessorStatus : (String -> msg) -> Sub msg


main =
    Browser.element
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        }



-- MODEL


type alias Model =
    Status


type Status
    = AwaitingInput
    | Processing
    | Done
    | Error ProcessingError


type ProcessingError
    = LessThanTwoImages


init : () -> ( Model, Cmd a )
init _ =
    ( AwaitingInput, Cmd.none )



-- UPDATE


type Msg
    = GotFiles (List D.Value)
    | ImageProcessorStatusUpdated


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotFiles files ->
            if List.length files >= 2 then
                ( Processing, sendImagesToJs <| List.take 2 files )

            else
                ( Error LessThanTwoImages, Cmd.none )

        ImageProcessorStatusUpdated ->
            ( Done, Cmd.none )


subscriptions _ =
    -- For now we only really handle the happy path, hence `always`.
    imageProcessorStatus (always ImageProcessorStatusUpdated)



-- VIEW


view model =
    let
        status =
            case model of
                Processing ->
                    p [] [ text "Processing ", span [ class "rotate" ] [ text "🌝" ] ]

                Error LessThanTwoImages ->
                    p [] [ text "Can't make a collage with just one image" ]

                _ ->
                    text ""
    in
    div []
        [ case model of
            Processing ->
                text ""

            _ ->
                fileInput
        , status
        ]


fileInput =
    input
        [ type_ "file"
        , accept "image/*"
        , multiple True
        , on "change" (D.map GotFiles filesDecoder)
        ]
        []


filesDecoder : D.Decoder (List D.Value)
filesDecoder =
    D.at [ "target", "files" ] (D.list D.value)
