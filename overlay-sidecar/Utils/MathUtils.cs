using System.Numerics;
using Valve.VR;

namespace AvatarSwitcher.OverlaySidecar.Utils;

/// <summary>
/// OpenVR の <see cref="HmdMatrix34_t"/>（3x4 row-major）と
/// System.Numerics の <see cref="Matrix4x4"/>（4x4 column-major寄りのレイアウト）を
/// 相互変換する。転置を誤ると「パネルが背後に出る/回転がおかしい」という
/// コンパイルエラーにならないバグになるため、値の対応は
/// 参照実装 (OyasumiVR src-overlay-sidecar/Utils/MathUtils.cs) の変換式をそのまま踏襲している。
/// </summary>
internal static class MathUtils
{
    public static HmdMatrix34_t ToHmdMatrix34T(this Matrix4x4 matrix)
    {
        return new HmdMatrix34_t
        {
            m0 = matrix.M11,
            m1 = matrix.M21,
            m2 = matrix.M31,
            m3 = matrix.M41,

            m4 = matrix.M12,
            m5 = matrix.M22,
            m6 = matrix.M32,
            m7 = matrix.M42,

            m8 = matrix.M13,
            m9 = matrix.M23,
            m10 = matrix.M33,
            m11 = matrix.M43,
        };
    }

    public static Matrix4x4 ToMatrix4X4(this HmdMatrix34_t matrix)
    {
        return new Matrix4x4(
            matrix.m0, matrix.m4, matrix.m8, 0,
            matrix.m1, matrix.m5, matrix.m9, 0,
            matrix.m2, matrix.m6, matrix.m10, 0,
            matrix.m3, matrix.m7, matrix.m11, 1
        );
    }

    public static HmdVector3_t ToHmdVector3T(this Vector3 vector)
    {
        return new HmdVector3_t { v0 = vector.X, v1 = vector.Y, v2 = vector.Z };
    }

    public static Vector3 ToVector3(this HmdVector3_t vector)
    {
        return new Vector3(vector.v0, vector.v1, vector.v2);
    }

    public static Vector2 ToVector2(this HmdVector2_t vector)
    {
        return new Vector2(vector.v0, vector.v1);
    }

    /// <summary>
    /// 変換行列が向いている方向の正規化ベクトルを返す（-Z 方向 = OpenVR の前方）。
    /// </summary>
    public static Vector3 GetDirectionNormal(this Matrix4x4 matrix)
    {
        var rotation = Matrix4x4.CreateFromQuaternion(Quaternion.CreateFromRotationMatrix(matrix));
        var offset = Matrix4x4.CreateTranslation(0, 0, -1f);
        return Vector3.Normalize((offset * rotation).Translation);
    }
}
